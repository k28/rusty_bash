//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::{ShellCore,Feeder};
use nix::unistd;
use nix::unistd::ForkResult;
use nix::sys::wait;
use std::ffi::CString;
use std::process;
use std::env;
use std::path::Path;

pub struct Command {
    text: String,
    args: Vec<String>,
    cargs: Vec<CString>,
}

impl Command {
    pub fn exec(&mut self, _core: &mut ShellCore) {
        if self.text == "exit\n" {
            process::exit(0);
        }

        if self.args[0] == "cd" && self.args.len() > 1 {
            let path = Path::new(&self.args[1]);
            if env::set_current_dir(&path).is_err() {
                eprintln!("Cannot change directory");
            }
            return;
        }

        match unsafe{unistd::fork()} {
            Ok(ForkResult::Child) => {
                let err = unistd::execvp(&self.cargs[0], &self.cargs);
                println!("Failed to execute. {:?}", err);
                process::exit(127);
            },
            Ok(ForkResult::Parent {child} ) => {
                let _ = wait::waitpid(child, None);
            },
            Err(err) => panic!("Failed to fork. {}", err)
        }
    }

    pub fn parse(feeder: &mut Feeder, _core: &mut ShellCore) -> Option<Command> {
        let line = feeder.consume(feeder.remaining.len());
        let args: Vec<String> = line
            .trim_end()
            .split(' ')
            .map(|w| w.to_string())
            .collect();

        let cargs: Vec<CString> = args
            .iter()
            .map(|w| CString::new(w.clone()).unwrap())
            .collect();

        if args.len() > 0 { // 1個以上の単語があればCommandのインスタンスを作成して返す
            return Some( Command {text: line, args: args, cargs: cargs} );
        }else{
            return None; // そうでなければ何も返さない
        }
    }
}
