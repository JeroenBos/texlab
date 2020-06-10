use crate::features::prelude::*;
use futures::{
    future::{AbortHandle, Abortable, Aborted},
    lock::Mutex,
    prelude::*,
    stream,
};
use log::error;
use std::{
    collections::{HashMap, HashSet},
    io,
    path::Path,
    process::Stdio,
    sync::Arc,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};
use uuid::Uuid;

#[derive(Debug)]
pub struct BuildEngine<C> {
    client: Arc<C>,
    handles_by_token: Mutex<HashMap<ProgressToken, AbortHandle>>,
    current_docs: Mutex<HashSet<Uri>>,
}

impl<C: LspClient + Send + Sync + 'static> BuildEngine<C> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            handles_by_token: Mutex::default(),
            current_docs: Mutex::default(),
        }
    }

    pub async fn is_busy(&self) -> bool {
        !self.current_docs.lock().await.is_empty()
    }

    pub async fn cancel(&self, token: ProgressToken) {
        let handles_by_token = self.handles_by_token.lock().await;
        if let Some(handle) = handles_by_token.get(&token) {
            handle.abort();
        } else if let ProgressToken::String(id) = token {
            if id == "texlab-build-*" {
                handles_by_token.values().for_each(|handle| handle.abort());
            }
        }
    }

    pub async fn execute(&self, ctx: &FeatureContext<BuildParams>) -> BuildResult {
        let token = ProgressToken::String(format!("texlab-build-{}", Uuid::new_v4()));
        let (handle, reg) = AbortHandle::new_pair();
        {
            let mut handles_by_token = self.handles_by_token.lock().await;
            handles_by_token.insert(token.clone(), handle);
        }

        let doc = ctx
            .snapshot()
            .parent(&ctx.current().uri, &ctx.options, &ctx.current_dir)
            .unwrap_or_else(|| Arc::clone(&ctx.view.current));

        if !doc.is_file() {
            error!("Unable to build the document {}: wrong URI scheme", doc.uri);
            return BuildResult {
                status: BuildStatus::Failure,
            };
        }

        {
            let mut current_docs = self.current_docs.lock().await;
            if current_docs.get(&doc.uri).is_some() {
                return BuildResult {
                    status: BuildStatus::Success,
                };
            }
            current_docs.insert(doc.uri.clone());
        }

        let status = match doc.uri.to_file_path() {
            Ok(path) => {
                if ctx.client_capabilities.has_work_done_progress_support() {
                    let params = WorkDoneProgressCreateParams {
                        token: token.clone(),
                    };
                    self.client.work_done_progress_create(params).await.unwrap();

                    let title = path.file_name().unwrap().to_string_lossy().into_owned();
                    let params = ProgressParams {
                        token: token.clone(),
                        value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                            WorkDoneProgressBegin {
                                title,
                                cancellable: Some(true),
                                message: Some("Building".into()),
                                percentage: None,
                            },
                        )),
                    };
                    self.client.progress(params).await;
                }

                let latex_options = ctx.options.latex.clone().unwrap_or_default();
                let client = Arc::clone(&self.client);
                match Abortable::new(build(&path, &latex_options, client), reg).await {
                    Ok(Ok(true)) => BuildStatus::Success,
                    Ok(Ok(false)) => BuildStatus::Error,
                    Ok(Err(why)) => {
                        error!("Unable to build the document {}: {}", doc.uri, why);
                        BuildStatus::Failure
                    }
                    Err(Aborted) => BuildStatus::Cancelled,
                }
            }
            Err(()) => {
                error!("Unable to build the document {}: invalid URI", doc.uri);
                BuildStatus::Failure
            }
        };

        if ctx.client_capabilities.has_work_done_progress_support() {
            let params = ProgressParams {
                token: token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                    message: None,
                })),
            };
            self.client.progress(params).await;
        }
        {
            let mut handles_by_token = self.handles_by_token.lock().await;
            handles_by_token.remove(&token);
        }

        {
            self.current_docs.lock().await.remove(&doc.uri);
        }
        BuildResult { status }
    }
}

async fn build<C>(path: &Path, options: &LatexOptions, client: Arc<C>) -> io::Result<bool>
where
    C: LspClient + Send + Sync + 'static,
{
    let build_options = options.build.as_ref().cloned().unwrap_or_default();
    let build_dir = options
        .root_directory
        .as_ref()
        .map(AsRef::as_ref)
        .or_else(|| path.parent())
        .unwrap();

    let args: Vec<_> = build_options
        .args()
        .into_iter()
        .map(|arg| replace_placeholder(arg, path))
        .collect();

    let mut process = Command::new(build_options.executable())
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(build_dir)
        .kill_on_drop(true)
        .spawn()?;

    let stdout = BufReader::new(process.stdout.take().unwrap()).lines();
    let stderr = BufReader::new(process.stderr.take().unwrap()).lines();
    let mut output = stream::select(stdout, stderr);

    tokio::spawn(async move {
        while let Some(Ok(line)) = output.next().await {
            let params = LogMessageParams {
                typ: MessageType::Log,
                message: line,
            };

            client.log_message(params).await;
        }
    });

    Ok(process.await?.success())
}

fn replace_placeholder(arg: String, file: &Path) -> String {
    if arg.starts_with('"') || arg.ends_with('"') {
        arg
    } else {
        arg.replace("%f", &file.to_string_lossy())
    }
}
