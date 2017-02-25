#![feature(core_intrinsics)]

extern crate xcb;
extern crate getopts;

use std::fmt::*;
use std::env;
use xcb::xproto;

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
	//opts.optflag("f", "filter", "filter rule");
	opts.optflag("o", "omit-hidden", "omit hidden windows");
	opts.optflag("h", "help", "show this help");
	opts.optflag("n", "num", "show event sequence count");

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

    if args.opt_present("m") {
        wm::monitor(&c, &screen, &args);
    } else {
        let windows = wm::collect_windows(&c, &args);
        wm::dump_windows(&windows, &args);
    }
}

