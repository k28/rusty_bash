//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::{ShellCore, Feeder};
use crate::abst_script_elem::ScriptElem;
use std::os::unix::prelude::RawFd;
use crate::elem_script::Script;
use crate::elem_redirect::Redirect;
use crate::elem_arg_delimiter::ArgDelimiter;
use nix::unistd::{close, fork, Pid, ForkResult};
use std::process::exit;
use crate::utils_io::set_child_io;
use crate::elem_end_of_command::Eoc;

/* ( script ) */
pub struct CompoundIf {
    pub ifthen: Vec<(Script, Script)>,
    pub else_do: Option<Script>,
    pub text: String,
    pub pid: Option<Pid>,
    pub redirects: Vec<Box<Redirect>>,
    pub pipein: RawFd,
    pub pipeout: RawFd,
    pub prevpipein: RawFd,
    pub eoc: Option<Eoc>,
}

impl ScriptElem for CompoundIf {
    fn exec(&mut self, conf: &mut ShellCore) {
        unsafe {
            match fork() {
                Ok(ForkResult::Child) => {
                    set_child_io(self.pipein, self.pipeout, self.prevpipein, &self.redirects);
                    self.exec_if_compound(conf);
                    close(1).expect("Can't close a pipe end");
                    exit(0);
                },
                Ok(ForkResult::Parent { child } ) => {
                    self.pid = Some(child);
                    return;
                },
                Err(err) => panic!("Failed to fork. {}", err),
            }
        }
    }

    fn get_pid(&self) -> Option<Pid> { self.pid }

    fn set_pipe(&mut self, pin: RawFd, pout: RawFd, pprev: RawFd) {
        self.pipein = pin;
        self.pipeout = pout;
        self.prevpipein = pprev;
    }

    fn get_pipe_end(&mut self) -> RawFd { self.pipein }
    fn get_pipe_out(&mut self) -> RawFd { self.pipeout }

    fn get_eoc_string(&mut self) -> String {
        if let Some(e) = &self.eoc {
            return e.text.clone();
        }

        "".to_string()
    }

    fn get_text(&self) -> String { self.text.clone() }
}

impl CompoundIf {
    pub fn new() -> CompoundIf{
        CompoundIf {
            ifthen: vec!(),
            else_do: None,
            redirects: vec!(),
            text: "".to_string(),
            pipein: -1,
            pipeout: -1,
            prevpipein: -1,
            pid: None,
            eoc: None,
        }
    }

    fn exec_if_compound(&mut self, conf: &mut ShellCore) {
        for pair in self.ifthen.iter_mut() {
             pair.0.exec(conf);
             if conf.vars["?"] != "0" {
                //conf.vars.insert("?".to_string(), "0".to_string());
                continue;
             }
             pair.1.exec(conf);
             return;
        }
    }

    fn parse_if_then_pair(text: &mut Feeder, conf: &mut ShellCore) -> Option<((Script, Script), String)> {
        let mut ans_text = String::new();

        let cond = if let Some(s) = Script::parse(text, conf, true) {
            ans_text += &s.text;
            s
        }else{
            //text.rewind(backup);
            return None;
        };

        if let Some(d) = ArgDelimiter::parse(text){
            ans_text += &d.text;
        }

        if text.compare(0, "then"){
            ans_text += &text.consume(4);
        }

        let doing = if let Some(s) = Script::parse(text, conf, true) {
            ans_text += &s.text;
            s
        }else{
            //text.rewind(backup);
            return None;
        };

        Some( ((cond, doing), ans_text) )
    }

    pub fn parse(text: &mut Feeder, conf: &mut ShellCore) -> Option<CompoundIf> {
        if text.len() < 2 || ! text.compare(0, "if") {
            return None;
        }

        let backup = text.clone();

        let mut ans = CompoundIf::new();
        ans.text += &text.consume(2);

        loop {
            if let Some(if_then_str) = CompoundIf::parse_if_then_pair(text, conf) {
                ans.text += &if_then_str.1;
                ans.ifthen.push(if_then_str.0);
            }else{
                text.rewind(backup);
                return None;
            }
    
            if text.compare(0, "fi"){
                ans.text += &text.consume(2);
                break;
            }else if text.compare(0, "elif"){
                ans.text += &text.consume(4);
            }else{
                text.rewind(backup);
                return None;
            }
        }

        if let Some(d) = ArgDelimiter::parse(text){
            ans.text += &d.text;
        }

        if let Some(e) = Eoc::parse(text){
            ans.text += &e.text;
            ans.eoc = Some(e);
        }

        //ans.ifthen.push((cond, doing));
        Some(ans)
    }
}