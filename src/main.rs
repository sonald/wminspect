//#![feature(core_intrinsics)]

extern crate xcb;
extern crate clap;

#[macro_use]
extern crate serde_derive;
extern crate serde;


use std::collections::HashSet;
use clap::{Arg, App, SubCommand};
mod wm;

pub fn main() {
    let matches = App::new("window manager inspector")
        .args(&[
              Arg::with_name("only-mapped").short("v").long("only-mapped").help("show only mapped windows"),
              Arg::from_usage("-c --colored 'output info with color'"),
              Arg::from_usage("-m --monitor 'run in monitor mode.'"),
              Arg::from_usage("-f --filter [RULE EXPR] 'filter rule.'"),
              Arg::from_usage("-o --omit-hidden 'omit hidden windows'"),
              Arg::from_usage("-O --no-override-redirect 'ignore override-redirect windows'"),
              Arg::from_usage("-s --no-special 'ignore special windows'"),
              Arg::from_usage("-n --num 'show event sequence count'"),
              Arg::from_usage("-d --diff 'highlight diffs between events'"),
              Arg::from_usage("--clients-only 'trace clients of window manager only'"),
              Arg::from_usage("--show-grammar 'show detailed grammar for filter rule'"),
        ])
        .subcommand(SubCommand::with_name("monitor").about("the same as -m flag"))
        .subcommand(
            SubCommand::with_name("sheet").about("sheet management")
            .args(&[
                  Arg::from_usage("--load [SHEET_PATH] 'load sheet from file, could be a .json, .bin or raw unparsed .rule'"),
                  Arg::from_usage("--compile [rule] [binfile|json] 'compile .rule into .bin or .json'").conflicts_with("load")
            ]))
        .get_matches();

    if matches.is_present("show-grammar") {
        println!("{}", wm::filter_grammar());
        return;
    }


    let (c, _) = xcb::Connection::connect(None).unwrap();
    let screen = c.get_setup().roots().next().unwrap();

    let mut f = match matches.value_of("filter") {
        None => wm::Filter::new(),
        Some(rule) => wm::Filter::parse(rule)
    };

    if let Some(sub) = matches.subcommand_matches("sheet") {
        if let Some(vals) = sub.values_of("compile") {
            let vals = vals.collect::<Vec<&str>>();
            wm::Filter::compile(vals[0], vals[1]);
            return;
        } 

        if let Some(val) = sub.value_of("load") {
            f.load_sheet(val);
        }
    }

    if matches.is_present("only-mapped") { f.set_mapped_only(); }
    if matches.is_present("colored") { f.set_colorful(); }
    if matches.is_present("omit-hidden") { f.set_omit_hidden(); }
    if matches.is_present("no-special") { f.set_no_special(); }
    if matches.is_present("diff") { f.set_show_diff(); }

    if matches.is_present("monitor") || matches.subcommand_matches("monitor").is_some() {
        wm::monitor(&c, &screen, &f);
    } else {
        let ctx = wm::Context::new(&c);
        let windows = ctx.collect_windows(&f);
        wm::dump_windows(&windows, &f, &HashSet::new());
    }
}

