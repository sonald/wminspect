//#![feature(core_intrinsics)]

extern crate xcb;
extern crate clap;

use std::collections::HashSet;
use clap::{Arg, App, SubCommand};
mod wm;

pub fn main() {
    let matches = App::new("window manager inspector")
        .args(&[
              Arg::with_name("only-mapped").short("v").long("only-mapped").help("show only mapped windows"),
              Arg::from_usage("-c --colored 'output info with color'"),
              Arg::from_usage("-m --monitor 'run in monitor mode.'"),
              Arg::from_usage("-f --filter [rule expr] 'filter rule.'"),
              Arg::from_usage("-o --omit-hidden 'omit hidden windows'"),
              Arg::from_usage("-O --no-override-redirect 'ignore override-redirect windows'"),
              Arg::from_usage("-s --no-special 'ignore special windows'"),
              Arg::from_usage("-n --num 'show event sequence count'"),
              Arg::from_usage("-d --diff 'highlight diffs between events'"),
              Arg::from_usage("--show-grammar 'show detailed grammar for filter rule'"),
        ])
        .subcommand(SubCommand::with_name("monitor").about("the same as -m flag"))
        .get_matches();

    if matches.is_present("show-grammar") {
        println!("{}", wm::filter_grammar());
        return;
    }

    let (c, _) = xcb::Connection::connect(None).unwrap();
    let screen = c.get_setup().roots().next().unwrap();

    let rule = match matches.value_of("filter") {
        None => "".to_string(),
        Some(s) => s.to_string()
    };
    let mut f = wm::parse_filter(rule);
    if matches.is_present("only-mapped") { f.set_mapped_only(); }
    if matches.is_present("colored") { f.set_colorful(); }
    if matches.is_present("omit-hidden") { f.set_omit_hidden(); }
    if matches.is_present("no-special") { f.set_no_special(); }
    if matches.is_present("diff") { f.set_show_diff(); }

    if matches.is_present("monitor") || matches.subcommand_matches("monitor").is_some(){
        wm::monitor(&c, &screen, &f);
    } else {
        let windows = wm::collect_windows(&c, &f);
        wm::dump_windows(&windows, &f, &HashSet::new());
    }
}

