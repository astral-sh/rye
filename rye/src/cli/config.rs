use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use clap::Args as ClapArgs;
use clap::Parser;
use clap::ValueEnum;
use serde::Serialize;
use toml_edit::value;
use toml_edit::Item;
use toml_edit::Table;
use toml_edit::Value;

use crate::config::Config;

#[derive(ValueEnum, Copy, Clone, Serialize, Debug, PartialEq)]
#[value(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
enum Format {
    Json,
}

/// Reads or modifies the global `config.toml` file.
///
/// The config file can be read via `--get` and it can be set with one
/// of the set options (`--set`, `--set-int`, `--set-bool`, or `--unset`).
/// Each of the set operations takes a key=value pair. All of these can
/// be supplied multiple times.
#[derive(Parser, Debug)]
#[command(arg_required_else_help(true))]
pub struct Args {
    #[command(flatten)]
    action: ActionArgs,
    /// Print the path to the config.
    #[arg(long, conflicts_with = "format")]
    show_path: bool,
    /// Request parseable output format rather than lines.
    #[arg(long)]
    format: Option<Format>,
}

#[derive(ClapArgs, Debug)]
#[group(required = true, multiple = true)]
pub struct ActionArgs {
    /// Reads a config key
    #[arg(long)]
    get: Vec<String>,
    /// Sets a config key to a string.
    #[arg(long)]
    set: Vec<String>,
    /// Sets a config key to an integer.
    #[arg(long)]
    set_int: Vec<String>,
    /// Sets a config key to a bool.
    #[arg(long)]
    set_bool: Vec<String>,
    /// Remove a config key.
    #[arg(long)]
    unset: Vec<String>,
}
pub fn execute(cmd: Args) -> Result<(), Error> {
    let mut config = Config::current();
    let doc = Arc::make_mut(&mut config).doc_mut();

    if cmd.show_path {
        echo!("{}", config.path().display());
        return Ok(());
    }

    let mut read_as_json = BTreeMap::new();
    let mut read_as_string = Vec::new();
    let reads = !cmd.action.get.is_empty();

    for item in cmd.action.get {
        let mut ptr = Some(doc.as_item());
        for piece in item.split('.') {
            ptr = ptr.as_ref().and_then(|x| x.get(piece));
        }

        let val = ptr.and_then(|x| x.as_value());
        match cmd.format {
            None => {
                read_as_string.push(value_to_string(val));
            }
            Some(Format::Json) => {
                read_as_json.insert(item, value_to_json(val));
            }
        }
    }

    let mut updates: Vec<(&str, Value)> = Vec::new();

    for item in &cmd.action.set {
        if let Some((key, value)) = item.split_once('=') {
            updates.push((key, Value::from(value)));
        } else {
            bail!("Invalid value for --set ({})", item);
        }
    }

    for item in &cmd.action.set_int {
        if let Some((key, value)) = item.split_once('=') {
            updates.push((
                key,
                Value::from(
                    value
                        .parse::<i64>()
                        .with_context(|| format!("Invalid value for --set-int ({})", item))?,
                ),
            ));
        } else {
            bail!("Invalid value for --set-int ({})", item);
        }
    }

    for item in &cmd.action.set_bool {
        if let Some((key, value)) = item.split_once('=') {
            updates.push((
                key,
                Value::from(
                    value
                        .parse::<bool>()
                        .with_context(|| format!("Invalid value for --set-bool ({})", item))?,
                ),
            ));
        } else {
            bail!("Invalid value for --set-bool ({})", item);
        }
    }

    let modifies = !updates.is_empty() || !cmd.action.unset.is_empty();
    if modifies && reads {
        bail!("cannot mix get and set operations");
    }

    for (key, new_value) in updates {
        let mut ptr = doc.as_item_mut();
        for piece in key.split('.') {
            if ptr.is_none() {
                let mut tbl = Table::new();
                tbl.set_implicit(true);
                *ptr = Item::Table(tbl);
            }
            ptr = &mut ptr[piece];
        }
        *ptr = value(new_value);
    }

    for key in cmd.action.unset {
        let mut ptr = doc.as_item_mut();
        if let Some((parent, key)) = key.rsplit_once('.') {
            for piece in parent.split('.') {
                ptr = &mut ptr[piece];
            }
            if let Some(tbl) = ptr.as_table_like_mut() {
                tbl.remove(key);
            }
            if let Item::Table(ref mut tbl) = ptr {
                if tbl.is_empty() {
                    tbl.set_implicit(true);
                }
            }
        } else {
            doc.remove(&key);
        }
    }

    if modifies {
        config.save()?;
    }

    match cmd.format {
        None => {
            for line in read_as_string {
                echo!("{}", line);
            }
        }
        Some(Format::Json) => {
            echo!("{}", serde_json::to_string_pretty(&read_as_json)?);
        }
    }

    Ok(())
}

fn value_to_json(val: Option<&Value>) -> serde_json::Value {
    match val {
        Some(Value::String(s)) => serde_json::Value::String(s.value().into()),
        Some(Value::Integer(i)) => serde_json::Value::Number((*i.value()).into()),
        Some(Value::Float(f)) => match serde_json::Number::from_f64(*f.value()) {
            Some(num) => serde_json::Value::Number(num),
            None => serde_json::Value::Null,
        },
        Some(Value::Boolean(b)) => serde_json::Value::Bool(*b.value()),
        Some(Value::Datetime(d)) => serde_json::Value::String(d.to_string()),
        Some(Value::Array(a)) => {
            serde_json::Value::Array(a.iter().map(|x| value_to_json(Some(x))).collect())
        }
        Some(Value::InlineTable(t)) => serde_json::Value::Object(
            t.iter()
                .map(|(k, v)| (k.to_string(), value_to_json(Some(v))))
                .collect(),
        ),
        None => serde_json::Value::Null,
    }
}

fn value_to_string(val: Option<&Value>) -> String {
    match val {
        Some(Value::String(s)) => s.value().to_string(),
        Some(Value::Integer(i)) => i.value().to_string(),
        Some(Value::Float(f)) => f.value().to_string(),
        Some(Value::Boolean(b)) => b.value().to_string(),
        Some(Value::Datetime(d)) => d.value().to_string(),
        Some(Value::Array(a)) => {
            let mut rv = String::from('[');
            for (idx, item) in a.iter().enumerate() {
                if idx > 0 {
                    rv.push_str(", ");
                }
                rv.push_str(&value_to_string(Some(item)));
            }
            rv.push(']');
            rv
        }
        Some(Value::InlineTable(t)) => {
            let mut rv = String::from('{');
            for (idx, (key, value)) in t.iter().enumerate() {
                if idx > 0 {
                    rv.push_str(", ");
                }
                rv.push_str(key);
                rv.push_str(" = ");
                rv.push_str(&value_to_string(Some(value)));
            }
            rv.push('}');
            rv
        }
        None => "?".into(),
    }
}
