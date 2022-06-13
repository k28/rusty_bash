//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::debuginfo::DebugInfo;
use crate::ShellCore;
use crate::Feeder;
use crate::scanner::*;

use crate::abst_arg_elem::ArgElem;
use crate::elem_subarg_non_quoted::SubArgNonQuoted;
use crate::elem_subarg_variable::SubArgVariable;
use crate::elem_subarg_command_expansion::SubArgCommandExp;


pub struct SubArgDoubleQuoted {
    pub text: String,
    pub pos: DebugInfo,
    pub subargs: Vec<Box<dyn ArgElem>>
}

impl ArgElem for SubArgDoubleQuoted {
    fn eval(&mut self, conf: &mut ShellCore) -> Vec<String> {
        let mut text = "".to_string();
        for a in &mut self.subargs {
            let sub = a.eval(conf);
            text += &sub[0];
        };

        let s = text.replace("\\", "\\\\").replace("*", "\\*"); 
        vec!(s)
    }

    fn text(&self) -> String {
        self.text.clone()
    }
}


impl SubArgDoubleQuoted {
/* parser for a string such as "aaa${var}" */
    pub fn parse(text: &mut Feeder, conf: &mut ShellCore) -> Option<SubArgDoubleQuoted> {
        let backup = text.clone();
    
        let mut ans = SubArgDoubleQuoted {
            text: "".to_string(),
            pos: DebugInfo::init(text),
            subargs: vec!(),
        };
    
        if scanner_until(text, 0, "\"") != 0 {
            return None;
        }
        text.consume(1);
    
        loop {
            if let Some(a) = SubArgVariable::parse2(text) {
                ans.subargs.push(Box::new(a));
            }else if let Some(a) = SubArgCommandExp::parse(text, conf) {
                ans.subargs.push(Box::new(a));
            }else if let Some(a) = SubArgVariable::parse(text) {
                ans.subargs.push(Box::new(a));
            }else if let Some(a) = SubArgNonQuoted::parse4(text) {
                ans.subargs.push(Box::new(a));
            }else{
                break;
            };
        }
    
        if scanner_until(text, 0, "\"") != 0 {
            text.rewind(backup);
            return None;
        }
        text.consume(1);
    
        let mut text = "\"".to_string();
        for a in &ans.subargs {
            text += &a.text();
        }
        ans.text = text + "\"";
    
        Some(ans)
    }
}
