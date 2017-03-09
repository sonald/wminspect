#![feature(core_intrinsics)]

extern crate xcb;
extern crate getopts;

use std::env;
use std::collections::HashSet;
mod wm;

fn usage(program: &String, opts: &getopts::Options)
{
    let brief = format!("Usage: {} [vcmof]", program);
    println!("{}", opts.usage(&brief));
}

pub fn main() {
    let args = env::args().collect::<Vec<String>>();
    let program = args[0].clone();

    let mut opts = getopts::Options::new();
    opts.optflag("v", "only-mapped", "show only mapped windows");
	opts.optflag("c", "colored", "output info with color");
	opts.optflag("m", "monitor", "run in monitor mode");
    opts.optopt("f", "filter", "filter rule", "\"RULE EXPR\"");
	opts.optflag("o", "omit-hidden", "omit hidden windows");
	opts.optflag("s", "no-special", "ignore special windows");
	opts.optflag("h", "help", "show this help");
	opts.optflag("n", "num", "show event sequence count");
	opts.optflag("d", "diff", "highlight diffs between events");

    let args = match opts.parse(&args) {
        Ok(m) => m,
        Err(_) => { usage(&program, &opts); return; },
    };

    if args.opt_present("h") {
        usage(&program, &opts);
        return;
    }

    let (c, _) = xcb::Connection::connect(None).unwrap();
    let screen = c.get_setup().roots().next().unwrap();

    let rule = match args.opt_str("f") {
        None => "".to_string(),
        Some(s) => s
    };
    let mut f = wm::parse_filter(rule);
    if args.opt_present("v") { f.set_mapped_only(); }
    if args.opt_present("c") { f.set_colorful(); }
    if args.opt_present("o") { f.set_omit_hidden(); }
    if args.opt_present("s") { f.set_no_special(); }
    if args.opt_present("d") { f.set_show_diff(); }

    if args.opt_present("m") {
        wm::monitor(&c, &screen, &f);
    } else {
        let windows = wm::collect_windows(&c, &f);
        wm::dump_windows(&windows, &f, HashSet::new());
    }
}

