//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::ShellCore;
use crate::utils::{eval_glob, combine};
use crate::debuginfo::DebugInfo;
use crate::elems_in_arg::{VarName, ArgElem};
use crate::Feeder;

pub trait CommandPart {
    fn parse_info(&self) -> Vec<String>;
    fn eval(&mut self, _conf: &mut ShellCore) -> Vec<String> { vec!() }
    fn text(&self) -> String { String::new() }
}

/* delimiter */
#[derive(Debug)]
pub struct ArgDelimiter {
    pub text: String,
    pub debug: DebugInfo,
}

impl CommandPart for ArgDelimiter {
    fn parse_info(&self) -> Vec<String> {
        vec!(format!("    delimiter: '{}' ({})", self.text.clone(), self.debug.text()))
    }

    fn text(&self) -> String { self.text.clone() }
}

impl ArgDelimiter{
    pub fn return_if_valid(text: &mut Feeder, pos: usize) -> Option<ArgDelimiter> {
        if pos == 0 {
            return None;
        };

        Some(ArgDelimiter{text: text.consume(pos), debug: DebugInfo::init(text)})
    }
}

/* ;, \n, and comment */
#[derive(Debug)]
pub struct Eoc {
    pub text: String,
    pub debug: DebugInfo,
}

impl CommandPart for Eoc {
    fn parse_info(&self) -> Vec<String> {
        vec!(format!("    end mark : '{}' ({})\n", self.text.clone(), self.debug.text()))
    }

    fn text(&self) -> String { self.text.clone() }
}

pub struct Substitution {
    pub text: String,
    pub name: VarName,
    pub value: Arg,
    pub debug: DebugInfo,
}

impl Substitution {
    pub fn new(text: &Feeder, name: VarName, value: Arg) -> Substitution{
        Substitution {
            text: name.text.clone() + "=" + &value.text.clone(),
            name: name, 
            value: value,
            debug: DebugInfo::init(text)
        }
    }
}

impl CommandPart for Substitution {
    fn parse_info(&self) -> Vec<String> {
        vec!(format!("    substitution: '{}' ({})\n", self.text.clone(), self.debug.text()))
    }

    fn eval(&mut self, conf: &mut ShellCore) -> Vec<String> { 
        let mut ans = vec!();
        ans.push(self.name.text.clone());
        
        let mut v = "".to_string();
        for s in self.value.eval(conf){
            v += &s;
        }
        ans.push(v);

        ans
    }

    fn text(&self) -> String { self.text.clone() }
}

pub struct Arg {
    pub text: String,
    pub pos: DebugInfo,
    pub subargs: Vec<Box<dyn ArgElem>>
}

impl Arg {
    pub fn expand_glob(text: &String) -> Vec<String> {
        let mut ans = eval_glob(text);

        if ans.len() == 0 {
            let s = text.clone().replace("\\*", "*").replace("\\\\", "\\");
            ans.push(s);
        };
        ans
    }

    pub fn remove_escape(text: &String) -> String{
        let mut escaped = false;
        let mut ans = "".to_string();
        
        for ch in text.chars() {
            if escaped || ch != '\\' {
                ans.push(ch);
            };
            escaped = !escaped && ch == '\\';
        }
        ans
    }
}

impl CommandPart for Arg {
    fn parse_info(&self) -> Vec<String> {
        let mut ans = vec!(format!("    arg      : '{}' ({})",
                              self.text.clone(), self.pos.text()));
        for sub in &self.subargs {
            ans.push("        subarg      : ".to_owned() + &*sub.text());
        };

        ans
    }

    fn eval(&mut self, conf: &mut ShellCore) -> Vec<String> {
        let mut subevals = vec!();
        for sa in &mut self.subargs {
            subevals.push(sa.eval(conf));
        }

        if subevals.len() == 0 {
            return vec!();
        };

        let mut strings = vec!();
        for ss in subevals {
            strings = combine(&strings, &ss);
        }
        strings
    }

    fn text(&self) -> String { self.text.clone() }
}

pub struct Redirect {
    pub text: String,
    pub pos: DebugInfo,
    pub left_fd: i32,
    pub direction_str: String,
    pub path: String,
}
