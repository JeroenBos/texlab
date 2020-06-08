use crate::features::{
    outline::OutlineContext,
    prelude::*,
    symbol::{
        section::util::{label_name, selection_range},
        types::{LatexSymbol, LatexSymbolKind},
    },
};

pub fn find_symbols(view: &DocumentView, table: &latex::SymbolTable) -> Vec<LatexSymbol> {
    let mut symbols = Vec::new();
    for equation in &table.equations {
        symbols.push(make_symbol(view, table, equation.range(&table)));
    }

    for equation in &table.environments {
        if equation.left.is_math(&table) {
            symbols.push(make_symbol(view, table, equation.range(&table)));
        }
    }
    symbols
}

fn make_symbol(view: &DocumentView, table: &latex::SymbolTable, full_range: Range) -> LatexSymbol {
    let label = table.find_label_by_range(full_range);

    let name = match label.and_then(|label| OutlineContext::find_number(view, table, *label)) {
        Some(num) => format!("Equation ({})", num),
        None => "Equation".to_owned(),
    };

    LatexSymbol {
        name,
        label: label_name(table, label),
        kind: LatexSymbolKind::Equation,
        deprecated: false,
        full_range,
        selection_range: selection_range(table, full_range, label),
        children: Vec::new(),
    }
}
