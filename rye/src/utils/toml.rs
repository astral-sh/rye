use anyhow::{anyhow, bail, Error};
use toml_edit::{Array, DocumentMut, Item, RawString, Table, TableLike};

/// Given a toml document, ensures that a given named table exists toplevel.
///
/// The table is created as a non inline table which is the preferred style.
pub fn ensure_table<'a>(doc: &'a mut DocumentMut, name: &str) -> &'a mut Item {
    if doc.as_item().get(name).is_none() {
        let mut tbl = Table::new();
        tbl.set_implicit(true);
        doc.as_item_mut()[name] = Item::Table(tbl);
    }
    &mut doc.as_item_mut()[name]
}

/// Reformats a TOML array to multi line while trying to
/// preserve all comments and move them around.  This also makes
/// the array to have a trailing comma.
pub fn reformat_array_multiline(deps: &mut Array) {
    fn find_comments(s: Option<&RawString>) -> impl Iterator<Item = &str> {
        s.and_then(|x| x.as_str())
            .unwrap_or("")
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                line.starts_with('#').then_some(line)
            })
    }

    for item in deps.iter_mut() {
        let decor = item.decor_mut();
        let mut prefix = String::new();
        for comment in find_comments(decor.prefix()).chain(find_comments(decor.suffix())) {
            prefix.push_str("\n    ");
            prefix.push_str(comment);
        }
        prefix.push_str("\n    ");
        decor.set_prefix(prefix);
        decor.set_suffix("");
    }

    deps.set_trailing(&{
        let mut comments = find_comments(Some(deps.trailing())).peekable();
        let mut rv = String::new();
        if comments.peek().is_some() {
            for comment in comments {
                rv.push_str("\n    ");
                rv.push_str(comment);
            }
        }
        rv.push('\n');
        rv
    });
    deps.set_trailing_comma(true);
}

/// Iterate over tables in an array.
///
/// This helps one iterate over
pub fn iter_tables<'x>(
    item: &'x Item,
) -> Box<dyn Iterator<Item = Result<&'x dyn TableLike, Error>> + 'x> {
    if let Some(aot) = item.as_array_of_tables() {
        Box::new(aot.into_iter().map(|x| Ok(x as &dyn TableLike)))
    } else if let Some(arr) = item.as_array() {
        Box::new(arr.into_iter().map(|x| match x.as_inline_table() {
            Some(table) => Ok(table as &dyn TableLike),
            None => bail!("expected inline table, got {}", x.type_name()),
        }))
    } else {
        Box::new(
            Some(Err(anyhow!(
                "expected array of tables, got {}",
                item.type_name()
            )))
            .into_iter(),
        )
    }
}
