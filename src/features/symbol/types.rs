use crate::{
    features::prelude::*,
    syntax::{BibtexEntryTypeCategory, Structure},
};

#[derive(Debug, Clone)]
pub struct SymbolContext {
    pub inner: FeatureContext<DocumentSymbolParams>,
    pub items: Vec<LatexSymbol>,
}

impl SymbolContext {
    pub fn new(inner: FeatureContext<DocumentSymbolParams>) -> Self {
        Self {
            inner,
            items: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct WorkspaceSymbol {
    pub info: SymbolInformation,
    pub search_text: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LatexSymbolKind {
    Section,
    Figure,
    Algorithm,
    Table,
    Listing,
    Enumeration,
    EnumerationItem,
    Theorem,
    Equation,
    Entry(BibtexEntryTypeCategory),
    Field,
    String,
}

impl LatexSymbolKind {
    pub fn into_symbol_kind(self) -> SymbolKind {
        match self {
            Self::Section => Structure::Section.symbol_kind(),
            Self::Figure | Self::Algorithm | Self::Table | Self::Listing => {
                Structure::Float.symbol_kind()
            }
            Self::Enumeration => Structure::Environment.symbol_kind(),
            Self::EnumerationItem => Structure::Item.symbol_kind(),
            Self::Theorem => Structure::Theorem.symbol_kind(),
            Self::Equation => Structure::Equation.symbol_kind(),
            Self::Entry(category) => Structure::Entry(category).symbol_kind(),
            Self::Field => Structure::Field.symbol_kind(),
            Self::String => Structure::Entry(BibtexEntryTypeCategory::String).symbol_kind(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LatexSymbol {
    pub name: String,
    pub label: Option<String>,
    pub kind: LatexSymbolKind,
    pub deprecated: bool,
    pub full_range: Range,
    pub selection_range: Range,
    pub children: Vec<LatexSymbol>,
}

impl LatexSymbol {
    pub fn search_text(&self) -> String {
        let kind = match self.kind {
            LatexSymbolKind::Section => "latex section",
            LatexSymbolKind::Figure => "latex float figure",
            LatexSymbolKind::Algorithm => "latex float algorithm",
            LatexSymbolKind::Table => "latex float table",
            LatexSymbolKind::Listing => "latex float listing",
            LatexSymbolKind::Enumeration => "latex enumeration",
            LatexSymbolKind::EnumerationItem => "latex enumeration item",
            LatexSymbolKind::Theorem => "latex math",
            LatexSymbolKind::Equation => "latex math equation",
            LatexSymbolKind::Entry(_) => "bibtex entry",
            LatexSymbolKind::Field => "bibtex field",
            LatexSymbolKind::String => "bibtex string",
        };
        format!("{} {}", kind, self.name).to_lowercase()
    }

    pub fn flatten(mut self, buffer: &mut Vec<Self>) {
        if self.kind == LatexSymbolKind::Field {
            return;
        }
        for symbol in self.children.drain(..) {
            symbol.flatten(buffer);
        }
        buffer.push(self);
    }

    pub fn into_document_symbol(self) -> DocumentSymbol {
        let children = self
            .children
            .into_iter()
            .map(|child| child.into_document_symbol())
            .collect();

        DocumentSymbol {
            name: self.name,
            deprecated: Some(self.deprecated),
            detail: self.label,
            kind: self.kind.into_symbol_kind(),
            selection_range: self.selection_range,
            range: self.full_range,
            children: Some(children),
        }
    }

    pub fn into_symbol_info(self, uri: Uri) -> SymbolInformation {
        SymbolInformation {
            name: self.name,
            deprecated: Some(self.deprecated),
            kind: self.kind.into_symbol_kind(),
            container_name: None,
            location: Location::new(uri.into(), self.full_range),
        }
    }
}
