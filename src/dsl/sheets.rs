use std::path::Path;
use std::convert::AsRef;
use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt;
use std::fs::{File, create_dir_all};
use std::io::Read;
use super::filter::{scan_tokens, parse_rule, Filter, ActionFuncPair, FilterItem};
use crate::{wm_trace, wm_error};
extern crate serde_json;
extern crate bincode as bc;

#[derive(Debug, Clone, Copy)]
pub enum SheetFormat {
    Invalid,
    /// plain unparsed rule 
    Plain,
    /// serialized json format
    Json,
    /// serialized bincode format
    Binary
}

impl Filter {
    /// Extend filter with rules from `data` which can belong to any kind of `SheetFormat`
    pub fn extend_with<S: AsRef<str>>(&mut self, data: S, format: SheetFormat) -> &mut Self {
        #[inline]
        fn load_action_pairs<S: AsRef<str>>(rule: S) -> Option<Vec<FilterItem>> {
            let mut tokens = scan_tokens(rule);
            parse_rule(&mut tokens)
        }

        #[inline]
        fn load_bin_form(data: &str) -> Option<Vec<FilterItem>> {
            wm_trace!("load_bin_form");
            bincode::deserialize(data.as_bytes()).ok()
        }

        #[inline]
        fn load_json_form(data: &str) -> Option<Vec<FilterItem>> {
            wm_trace!("load_json_form");
            serde_json::from_str::<Vec<FilterItem>>(data).ok()
        }
        if let Some(items) = match format {
            SheetFormat::Json => load_json_form(data.as_ref()),
            SheetFormat::Binary => load_bin_form(data.as_ref()),
            SheetFormat::Plain => load_action_pairs(data.as_ref()),
            _ => None
        } {
            wm_trace!("extend_with {:?}", items);
            let mut items = items.into_iter()
                .map(|item| {
                     let f = item.rule.gen_closure();
                     ActionFuncPair { action: item.action, rule: item.rule, func: f }
                })
                .collect();
            self.rules.append(&mut items);
        }
        self
    }

    /// Load sheets from disk at `path`
    /// sheet may be in any of three forms: unparsed form with ext .rule,
    /// two serialized forms: .json and .bin
    ///
    pub fn load_sheet<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        if !path.as_ref().exists() {
            wm_error!("{:?} does not exists", path.as_ref());
            return self;
        }

        let ext: OsString = match path.as_ref().extension() {
            Some(ext) => OsString::from(ext),
            None => return self
        };


        if let Ok(mut f) = File::open(path.as_ref()) {
            let mut data = String::new();
            if let Err(_) = f.read_to_string(&mut data) {
                return self;
            }

            let format = match ext.as_bytes() {
                b"json" => SheetFormat::Json,
                b"bin" => SheetFormat::Binary,
                b"rule" => SheetFormat::Plain,
                _ => SheetFormat::Invalid
            };
            self.extend_with(&data, format);
        } else {
            wm_error!("load sheet from {:?} failed", path.as_ref());
        }

        self
    }

    /// Compile rule from disk file into json or bincode format
    pub fn compile<S: AsRef<Path>, P: AsRef<Path>>(rule: S, out: P) {
        wm_trace!("compile {:?} to {:?}", rule.as_ref(), out.as_ref());

        if !rule.as_ref().exists() {
            wm_error!("{:?} does not exists", rule.as_ref());
            return ;
        }

        if let Some(d) = out.as_ref().parent() {
            if !d.exists() && create_dir_all(d).is_err() {
                return;
            }
        }

        let ext: OsString = match out.as_ref().extension() {
            Some(ext) => OsString::from(ext),
            None => return
        };

        if let Ok(mut f) = File::open(rule.as_ref()) {
            let mut data = String::new();
            if let Err(_) = f.read_to_string(&mut data) {
                return;
            }

            let rule = {
                let mut tokens = scan_tokens(&data);
                parse_rule(&mut tokens)
            };

            let mut dest = match File::create(out.as_ref()) {
                Err(e) => {
                    wm_error!("create {:?} failed: {}", out.as_ref(), e);
                    return;
                }, 
                Ok(v) => v
            };

            let result = match (rule, ext.as_bytes()) {
                (Some(rule), b"json") => {
                    serde_json::to_writer(&mut dest, &rule)
                        .map_err(|e| format!("json: {}", e))
                },
                (Some(rule), b"bin") => {
                    bincode::serialize(&rule)
                        .map_err(|e| format!("bin: {}", e))
                        .and_then(|data| {
                            use std::io::Write;
                            dest.write_all(&data)
                                .map_err(|e| format!("write: {}", e))
                        })
                },
                _ => { Err("invalid extension".to_string()) }
            };

            if result.is_err() {
                wm_error!("compile failed: {}", result.err().unwrap());
            } else {
                wm_trace!("compile done");
            }
        }
    }
}


