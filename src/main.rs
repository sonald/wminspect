use clap::{Arg, Command};
use tracing::info;
pub mod core;
pub mod dsl;

use crate::core::tracing::init_tracing;

#[cfg(feature = "gui")]
mod ui;

#[cfg(any(
    feature = "x11",
    feature = "platform-linux",
    feature = "platform-windows",
    feature = "platform-macos"
))]
mod platform;

// For compatibility with existing code, we still export a temporary wm module
pub mod wm {
    pub use crate::core::*;
    pub use crate::dsl::*;
    #[cfg(feature = "x11")]
    pub use crate::platform::*;
}

pub fn main() {
    init_tracing();
    info!("Application started");
    let matches = Command::new("wminspect")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Window Manager Inspector Team")
        .about("A tool for inspecting X11 window manager state and monitoring window events")
        .long_about("wminspect is a comprehensive tool for inspecting X11 window manager state, \nmonitoring window events, and applying filtering rules to window collections.")
        .args(&[
            Arg::new("only-mapped").short('v').long("only-mapped")
                .help("Show only mapped windows (exclude unmapped/hidden)").action(clap::ArgAction::SetTrue),
            Arg::new("colored").short('c').long("colored")
                .help("Enable colored output for better readability").action(clap::ArgAction::SetTrue),
            Arg::new("concise").long("concise")
                .help("Use concise output format (less verbose)").action(clap::ArgAction::SetTrue),
            Arg::new("no-color").long("no-color")
                .help("Disable colored output").action(clap::ArgAction::SetTrue),
            Arg::new("monitor").short('m').long("monitor")
                .help("Run in monitor mode (watch for window events)").action(clap::ArgAction::SetTrue),
            Arg::new("filter").short('f').long("filter").value_name("RULE")
                .help("Apply filtering rules to window collection\n(use --show-grammar for rule syntax)"),
            Arg::new("preset").long("preset").value_name("NAME")
                .help("Load a built-in troubleshooting preset before applying --filter"),
            Arg::new("omit-hidden").short('o').long("omit-hidden")
                .help("Omit hidden/iconified windows from output").action(clap::ArgAction::SetTrue),
            Arg::new("no-override-redirect").short('O').long("no-override-redirect")
                .help("Ignore override-redirect windows (popups, tooltips)").action(clap::ArgAction::SetTrue),
            Arg::new("no-special").short('s').long("no-special")
                .help("Ignore special windows (docks, panels, etc.)").action(clap::ArgAction::SetTrue),
            Arg::new("num").short('n').long("num")
                .help("Show event sequence numbers in monitor mode").action(clap::ArgAction::SetTrue),
            Arg::new("diff").short('d').long("diff")
                .help("Highlight differences between consecutive events").action(clap::ArgAction::SetTrue),
            Arg::new("clients-only").short('C').long("clients-only")
                .help("Trace only client windows (managed by window manager)").action(clap::ArgAction::SetTrue),
            Arg::new("show-grammar").long("show-grammar")
                .help("Show detailed grammar for filter rule syntax").action(clap::ArgAction::SetTrue),
        ])
        .subcommand(
            Command::new("monitor")
                .about("Run in monitor mode (equivalent to -m flag)")
                .long_about("Monitor mode watches for window events in real-time,\nshowing changes as they occur.")
        )
        .subcommand(
            Command::new("sheet")
                .about("Manage filter rule sheets")
                .long_about("Sheet management allows you to load, compile, and manage\nfilter rule collections stored in various formats.")
                .args(&[
                    Arg::new("load").long("load").value_name("SHEET_PATH")
                        .help("Load filter rules from a sheet file\n(supports .json, .bin, or .rule formats)"),
                    Arg::new("compile").long("compile").value_names(["INPUT", "OUTPUT"])
                        .help("Compile .rule file into .bin or .json format")
                        .long_help("Compile a human-readable .rule file into a binary (.bin) or JSON (.json) format\nfor faster loading and processing.")
                        .conflicts_with("load").num_args(2)
                ])
                .subcommand(
                    Command::new("verify")
                        .about("Verify one sheet file or all supported sheets under a directory")
                        .arg(
                            Arg::new("path")
                                .value_name("PATH")
                                .required(true)
                                .help("Sheet file or directory to verify recursively"),
                        )
                        .arg(
                            Arg::new("detail")
                                .long("detail")
                                .help("Show detailed plain-text diagnostics")
                                .action(clap::ArgAction::SetTrue),
                        )
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Emit a detailed JSON report")
                                .conflicts_with("detail")
                                .action(clap::ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("builtin")
                        .about("List or show built-in troubleshooting sheet presets")
                        .subcommand_required(true)
                        .arg_required_else_help(true)
                        .subcommand(Command::new("list").about("List available built-in presets"))
                        .subcommand(
                            Command::new("show")
                                .about("Show the raw .rule text for one built-in preset")
                                .arg(
                                    Arg::new("name")
                                        .value_name("NAME")
                                        .required(true)
                                        .help("Built-in preset name"),
                                ),
                        ),
                )
        )
        .get_matches();

    if matches.get_flag("show-grammar") {
        println!("{}", dsl::filter_grammar());
        return;
    }

    let mut f = dsl::Filter::new();

    if let Some(sub) = matches.subcommand_matches("sheet") {
        if let Some(builtin) = sub.subcommand_matches("builtin") {
            if builtin.subcommand_matches("list").is_some() {
                for preset in dsl::builtin_sheet_presets() {
                    println!("{}\t{}", preset.name, preset.summary);
                }
                return;
            }

            if let Some(show) = builtin.subcommand_matches("show") {
                let name = show.get_one::<String>("name").unwrap();
                let Some(preset) = dsl::builtin_sheet_preset(name) else {
                    eprintln!(
                        "unknown built-in preset {:?}; available presets: {}",
                        name,
                        dsl::builtin_sheet_presets()
                            .iter()
                            .map(|preset| preset.name)
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    std::process::exit(1);
                };
                print!("{}", preset.rule_text);
                return;
            }
        }

        if let Some(verify) = sub.subcommand_matches("verify") {
            let path = verify.get_one::<String>("path").unwrap();
            let report = dsl::verify_target(path);

            if verify.get_flag("json") {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            } else if verify.get_flag("detail") {
                println!("{}", dsl::render_plain_detail(&report));
            } else {
                println!("{}", dsl::render_plain_summary(&report));
            }

            if report.has_failures() {
                std::process::exit(1);
            }
            return;
        }

        if let Some(vals) = sub.get_many::<String>("compile") {
            let vals: Vec<&str> = vals.map(|s| s.as_str()).collect();
            dsl::Filter::compile(vals[0], vals[1]);
            return;
        }

        if let Some(val) = sub.get_one::<String>("load") {
            f.load_sheet(val);
        }
    }

    if let Some(name) = matches.get_one::<String>("preset") {
        let Some(preset) = dsl::builtin_sheet_preset(name) else {
            eprintln!(
                "unknown built-in preset {:?}; available presets: {}",
                name,
                dsl::builtin_sheet_presets()
                    .iter()
                    .map(|preset| preset.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            std::process::exit(1);
        };

        f.extend_with(preset.rule_text, dsl::SheetFormat::Plain);
    }

    if let Some(rule) = matches.get_one::<String>("filter") {
        f.extend_with(rule, dsl::SheetFormat::Plain);
    }

    #[cfg(feature = "x11")]
    let (c, _) = xcb::Connection::connect(None).unwrap();
    #[cfg(feature = "x11")]
    let ewmh = xcb_util::ewmh::Connection::connect(c).unwrap_or_else(|_| {
        eprintln!("Failed to connect to X11 server");
        std::process::exit(1);
    });

    #[cfg(feature = "x11")]
    {
        let mut formatter = core::colorized_output::ColorizedFormatter::new();

        // Configure output mode
        if matches.get_flag("no-color") {
            formatter.set_mode(core::colorized_output::OutputMode::NoColor);
        } else if matches.get_flag("concise") {
            formatter.set_mode(core::colorized_output::OutputMode::Concise);
        } else if matches.get_flag("colored") {
            formatter.set_mode(core::colorized_output::OutputMode::Colorized);
        }

        let mut ctx = wm::Context::new_with_formatter(&ewmh, f, formatter);

        if matches.get_flag("only-mapped") {
            ctx.set_mapped_only();
        }
        if matches.get_flag("omit-hidden") {
            ctx.set_omit_hidden();
        }
        if matches.get_flag("no-special") {
            ctx.set_no_special();
        }
        if matches.get_flag("no-override-redirect") {
            ctx.set_no_override_redirect();
        }
        if matches.get_flag("diff") {
            ctx.set_show_diff();
        }
        if matches.get_flag("clients-only") {
            ctx.set_clients_only();
        }
        if matches.get_flag("num") {
            ctx.set_show_sequence_numbers();
        }
        if matches.get_flag("colored") {
            ctx.set_colorful();
        }
        if matches.get_flag("num") {
            ctx.set_show_sequence_numbers();
        }

        if matches.get_flag("monitor") || matches.subcommand_matches("monitor").is_some() {
            wm::monitor(&ctx);
        } else {
            ctx.refresh_windows();
            ctx.dump_windows(None);
        }
    }

    #[cfg(not(feature = "x11"))]
    {
        // Check if we're running on Wayland
        if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
            if session_type == "wayland" {
                println!("Running on Wayland session - X11 window inspection is not supported.");
                println!("This tool requires X11 to inspect window manager state.");
                println!("Consider running in an X11 session or using XWayland.");
                std::process::exit(0); // Graceful exit for Wayland
            }
        }

        // Check if we're on macOS
        if cfg!(target_os = "macos") {
            println!("Running on macOS - X11 support requires XQuartz.");
            println!(
                "Please install XQuartz and ensure it's running, then build with --features x11"
            );
            std::process::exit(1);
        }

        // Default error message for other platforms
        eprintln!("X11 support is required but not enabled. Please build with --features x11");
        std::process::exit(1);
    }
}
