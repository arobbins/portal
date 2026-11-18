use std::fmt::Debug;
use std::path::PathBuf;

use serde_json::{Value, json};

pub mod account;
pub mod account_event;
pub mod account_tab;
pub mod account_tab_sections;

pub mod admin_log;
pub mod admin_unlock;
pub mod api_token;
pub mod caller;
pub mod email_change;
pub mod emailed_link;
pub mod flash;
pub mod form_failure;

pub mod job;
pub mod known_device;
pub mod passkey_prompt;
pub mod passkey_prompt_grant;
pub mod passkey_reauth;
pub mod permission;
pub mod rate_limit;
pub mod role;
pub mod second_factor;

pub mod signed_in;
pub mod testing;
pub mod two_factor;

pub trait Labeled: Copy + Eq + Debug + 'static {
    fn id(self) -> &'static str;
    fn label(self) -> &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect<S> {
    Moves(S),
    Refused,
    Unreachable(&'static str),
}

pub struct Machine<S: 'static, E: 'static> {
    pub name: &'static str,
    pub summary: &'static str,
    pub states: &'static [S],
    pub terminal: &'static [S],
    pub events: &'static [E],
    pub stored_in: &'static [&'static str],
    pub rules: &'static [&'static str],
    pub writers: &'static [Writer<E>],
}

pub struct Writer<E: 'static> {
    pub caller: &'static str,
    pub query: &'static str,
    pub events: &'static [E],
}

pub struct CellResult<S, E> {
    pub from: S,
    pub event: E,
    pub expected: Expect<S>,
    pub actual: Option<S>,
    pub accepted: bool,
    pub broken: Vec<&'static str>,
}

impl<S: Labeled, E: Labeled> CellResult<S, E> {
    pub fn passed(&self) -> bool {
        self.broken.is_empty() && self.reached_expected_state()
    }

    fn reached_expected_state(&self) -> bool {
        match (self.expected, self.actual) {
            (Expect::Moves(to), Some(actual)) => self.accepted && actual == to,
            (Expect::Refused, Some(actual)) => actual == self.from,
            (Expect::Unreachable(_), None) => true,
            _ => false,
        }
    }

    fn to_json(&self) -> Value {
        let (expected, reason) = match self.expected {
            Expect::Moves(to) => (Some(to.id()), None),
            Expect::Refused => (Some(self.from.id()), None),
            Expect::Unreachable(reason) => (None, Some(reason)),
        };
        json!({
            "from": self.from.id(),
            "event": self.event.id(),
            "expected": expected,
            "refused": self.expected == Expect::Refused,
            "unreachable": reason,
            "actual": self.actual.map(Labeled::id),
            "accepted": self.accepted,
            "broken": self.broken,
            "passed": self.passed(),
        })
    }
}

fn labeled<T: Labeled>(items: &[T], terminal: &[T]) -> Vec<Value> {
    items
        .iter()
        .map(|item| {
            json!({ "id": item.id(), "label": item.label(), "terminal": terminal.contains(item) })
        })
        .collect()
}

fn writers_json<E: Labeled>(writers: &[Writer<E>]) -> Vec<Value> {
    writers
        .iter()
        .map(|writer| {
            json!({
                "caller": writer.caller,
                "query": writer.query,
                "events": writer.events.iter().map(|event| event.id()).collect::<Vec<_>>(),
            })
        })
        .collect()
}

pub fn write_results<S: Labeled, E: Labeled>(machine: &Machine<S, E>, cells: &[CellResult<S, E>]) {
    let report = json!({
        "name": machine.name,
        "summary": machine.summary,
        "states": labeled(machine.states, machine.terminal),
        "events": labeled(machine.events, &[]),
        "stored_in": machine.stored_in,
        "rules": machine.rules,
        "writers": writers_json(machine.writers),
        "cells": cells.iter().map(CellResult::to_json).collect::<Vec<_>>(),
    });
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/state-machines");
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join(format!(
        "{}.json",
        machine.name.to_lowercase().replace(' ', "-")
    ));
    std::fs::write(file, serde_json::to_string_pretty(&report).unwrap()).unwrap();
}

pub fn failures<S: Labeled, E: Labeled>(cells: &[CellResult<S, E>]) -> Vec<String> {
    cells
        .iter()
        .filter(|cell| !cell.passed())
        .map(|cell| {
            format!(
                "{} + {}: expected {:?}, got {:?} (accepted: {}), broken rules: {:?}",
                cell.from.label(),
                cell.event.label(),
                cell.expected,
                cell.actual,
                cell.accepted,
                cell.broken
            )
        })
        .collect()
}
