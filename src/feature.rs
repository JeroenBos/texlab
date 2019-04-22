use crate::workspace::{Document, Workspace};
use futures::stream::Fold;
use lsp_types::{
    DocumentLinkParams, FoldingRangeParams, Position, ReferenceContext, ReferenceParams,
    TextDocumentIdentifier, TextDocumentPositionParams,
};
use std::rc::Rc;
use url::Url;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FeatureRequest<T> {
    pub params: T,
    pub workspace: Workspace,
    pub document: Rc<Document>,
    pub related_documents: Vec<Rc<Document>>,
}

impl<T> FeatureRequest<T> {
    pub fn new(params: T, workspace: Workspace, document: Rc<Document>) -> Self {
        let related_documents = workspace.related_documents(&document.uri);
        FeatureRequest {
            params,
            workspace,
            document,
            related_documents,
        }
    }
}

#[cfg(test)]
pub struct FeatureTester {
    workspace: Workspace,
    document: Rc<Document>,
    document_id: TextDocumentIdentifier,
    position: Position,
    new_name: String,
}

#[cfg(test)]
impl FeatureTester {
    pub fn new(workspace: Workspace, uri: Url, line: u64, character: u64, new_name: &str) -> Self {
        let document = workspace.find(&uri).unwrap();
        FeatureTester {
            workspace,
            document,
            document_id: TextDocumentIdentifier::new(uri),
            position: Position::new(line, character),
            new_name: new_name.to_owned(),
        }
    }
}

#[cfg(test)]
impl Into<FeatureRequest<TextDocumentPositionParams>> for FeatureTester {
    fn into(self) -> FeatureRequest<TextDocumentPositionParams> {
        let params = TextDocumentPositionParams::new(self.document_id, self.position);
        FeatureRequest::new(params, self.workspace, self.document)
    }
}

#[cfg(test)]
impl Into<FeatureRequest<FoldingRangeParams>> for FeatureTester {
    fn into(self) -> FeatureRequest<FoldingRangeParams> {
        let params = FoldingRangeParams {
            text_document: self.document_id,
        };
        FeatureRequest::new(params, self.workspace, self.document)
    }
}

#[cfg(test)]
impl Into<FeatureRequest<DocumentLinkParams>> for FeatureTester {
    fn into(self) -> FeatureRequest<DocumentLinkParams> {
        let params = DocumentLinkParams {
            text_document: self.document_id,
        };
        FeatureRequest::new(params, self.workspace, self.document)
    }
}

#[cfg(test)]
impl Into<FeatureRequest<ReferenceParams>> for FeatureTester {
    fn into(self) -> FeatureRequest<ReferenceParams> {
        let params = ReferenceParams {
            text_document: self.document_id,
            position: self.position,
            context: ReferenceContext {
                include_declaration: false,
            },
        };
        FeatureRequest::new(params, self.workspace, self.document)
    }
}
