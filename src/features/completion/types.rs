use crate::{
    features::prelude::*,
    syntax::{AstNodeIndex, BibtexEntryTypeDoc, BibtexFieldDoc, Structure},
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use typed_builder::TypedBuilder;

#[derive(Debug, Clone)]
pub struct Item<'a> {
    pub range: Range,
    pub data: ItemData<'a>,
    pub preselect: bool,
    pub score: Option<i64>,
}

impl<'a> Item<'a> {
    pub fn new(range: Range, data: ItemData<'a>) -> Self {
        Self {
            range,
            data,
            preselect: false,
            score: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ItemData<'a> {
    ComponentCommand {
        name: &'a str,
        image: Option<&'a str>,
        glyph: Option<&'a str>,
        file_names: &'a [String],
    },
    ComponentEnvironment {
        name: &'a str,
        file_names: &'a [String],
    },
    UserCommand {
        name: &'a str,
    },
    UserEnvironment {
        name: &'a str,
    },
    Label {
        name: &'a str,
        kind: Structure,
        header: Option<String>,
        footer: Option<String>,
        text: String,
    },
    PgfLibrary {
        name: &'a str,
    },
    TikzLibrary {
        name: &'a str,
    },
    Package {
        name: Cow<'a, str>,
    },
    Class {
        name: Cow<'a, str>,
    },
    File {
        name: String,
    },
    Directory {
        name: String,
    },
    Citation {
        uri: &'a Uri,
        key: &'a str,
        text: String,
        ty: Structure,
    },
    Argument {
        name: &'a str,
        image: Option<&'a str>,
    },
    BeginCommand,
    Color {
        name: &'a str,
    },
    ColorModel {
        name: &'a str,
    },
    GlossaryEntry {
        name: &'a str,
    },
    EntryType {
        ty: &'a BibtexEntryTypeDoc,
    },
    Field {
        field: &'a BibtexFieldDoc,
    },
}

impl<'a> ItemData<'a> {
    pub fn label<'b: 'a>(&'b self) -> &'b str {
        match self {
            Self::ComponentCommand { name, .. } => name,
            Self::ComponentEnvironment { name, .. } => name,
            Self::UserCommand { name } => name,
            Self::UserEnvironment { name } => name,
            Self::Label { name, .. } => name,
            Self::Class { name } => &name,
            Self::Package { name } => &name,
            Self::PgfLibrary { name } => name,
            Self::TikzLibrary { name } => name,
            Self::File { name } => &name,
            Self::Directory { name } => &name,
            Self::Citation { key, .. } => key,
            Self::Argument { name, .. } => name,
            Self::BeginCommand => "begin",
            Self::Color { name } => name,
            Self::ColorModel { name } => name,
            Self::GlossaryEntry { name } => name,
            Self::EntryType { ty } => &ty.name,
            Self::Field { field } => &field.name,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompletionItemData {
    Command,
    CommandSnippet,
    Environment,
    Label,
    Folder,
    File,
    PgfLibrary,
    TikzLibrary,
    Color,
    ColorModel,
    Package,
    Class,
    EntryType,
    FieldName,
    Citation { uri: Uri, key: String },
    Argument,
    GlossaryEntry,
}

impl Into<serde_json::Value> for CompletionItemData {
    fn into(self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LatexArgument {
    pub cmd_node: AstNodeIndex,
    pub arg_node: AstNodeIndex,
    pub index: usize,
    pub word: bool,
}

impl LatexArgument {
    pub fn range(self, tree: &latex::Tree, pos: Position) -> Range {
        tree.children(self.arg_node)
            .filter_map(|child| tree.as_text(child))
            .flat_map(|text| text.words.iter())
            .map(|word| word.range())
            .find(|range| range.contains(pos))
            .unwrap_or_else(|| Range::new(pos, pos))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, TypedBuilder)]
pub struct LatexArgumentPattern<'a> {
    tree: &'a latex::Tree,
    name: &'a str,
    index: usize,
    position: Position,

    #[builder(default = false)]
    word: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum LatexCompletionScope {
    Command(AstNodeIndex),
    Argument(LatexArgument),
}

impl LatexCompletionScope {
    pub fn match_argument(self, pattern: LatexArgumentPattern) -> Option<(Range, AstNodeIndex)> {
        if let Self::Argument(arg) = self {
            let cmd = pattern.tree.as_command(arg.cmd_node).unwrap();
            if &cmd.name.text()[1..] == pattern.name
                && arg.index == pattern.index
                && (!pattern.word || arg.word)
            {
                Some((arg.range(pattern.tree, pattern.position), arg.cmd_node))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn match_environment(
        self,
        tree: &latex::Tree,
        pos: Position,
    ) -> Option<(Range, AstNodeIndex)> {
        LANGUAGE_DATA
            .environment_commands
            .iter()
            .filter_map(|cmd| {
                self.match_argument(
                    LatexArgumentPattern::builder()
                        .tree(tree)
                        .name(&cmd.name[1..])
                        .index(cmd.index)
                        .position(pos)
                        .build(),
                )
            })
            .next()
    }
}
