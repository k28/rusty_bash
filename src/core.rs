//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

pub mod builtins;
pub mod shopts;
pub mod job;

use std::collections::HashMap;
use std::fs::File;
use std::env;
use crate::core::shopts::Shopts;
use crate::core::job::Job;
use nix::sys::wait::{waitpid, WaitStatus, WaitPidFlag};
use nix::unistd::Pid;

use nix::unistd::read;
use std::os::unix::prelude::RawFd;

pub struct ShellCore {
    pub builtins: HashMap<String, fn(&mut ShellCore, args: &mut Vec<String>) -> i32>,
    pub functions: HashMap<String, String>,
    pub arrays: HashMap<String, Vec<String>>,
    pub vars: HashMap<String, String>,
    pub args: Vec<String>,
    pub aliases: HashMap<String, String>,
    pub history: Vec<String>,
    pub flags: String,
    pub jobs: Vec<Job>, // jobs[0]: foreground job, jobs[1:]: background jobs
    pub in_double_quot: bool,
    pub pipeline_end: String,
    pub script_file: Option<File>,
    pub return_enable: bool,
    pub return_flag: bool,
    pub shopts: Shopts, 
}

impl ShellCore {
    pub fn new() -> ShellCore {
        let mut conf = ShellCore{
            builtins: HashMap::new(),
            functions: HashMap::new(),
            arrays: HashMap::new(),
            vars: HashMap::new(),
            args: vec![],
            aliases: HashMap::new(),
            history: Vec::new(),
            flags: String::new(),
            jobs: vec!(Job::new(&"".to_string(), &vec![], false)),
            in_double_quot: false,
            pipeline_end: String::new(),
            script_file: None,
            return_flag: false,
            return_enable: false,
            shopts: Shopts::new(),
        };

        conf.set_var("?", &0.to_string());
        builtins::set_builtins(&mut conf);

        conf
    }

    pub fn set_var(&mut self, key: &str, value: &str) {
        self.vars.insert(key.to_string(), value.to_string());
    }

    pub fn get_var(&self, key: &str) -> String {
        if let Ok(n) = key.parse::<usize>() {
            if self.args.len() > n {
                return self.args[n].clone();
            }
        }

        if key == "-" {
            return self.flags.clone();
        }

        if key == "#" {
            return (self.args.len() - 1).to_string();
        }

        if key == "@" {
            if self.args.len() == 1 {
                return "".to_string();
            }

            return self.args[1..].to_vec().join(" ");
        }

        if key == "*" {
            if self.args.len() == 1 {
                return "".to_string();
            }

            if self.in_double_quot {
                if let Some(ch) = self.get_var("IFS").chars().nth(0){
                    return self.args[1..].to_vec().join(&ch.to_string());
                }
            }

            return self.args[1..].to_vec().join(" ");
        }

        if let Some(s) = self.vars.get(&key as &str){
            return s.to_string();
        };

        if let Ok(s) = env::var(&key) {
            return s.to_string();
        };

        "".to_string()
    }

    pub fn get_function(&mut self, name: &String) -> Option<String> {
        if self.functions.contains_key(name) {
            if let Some(s) = self.functions.get(name) {
                return Some(s.clone());
            }
        }

        None
    }

    pub fn get_builtin(&self, name: &String) 
        -> Option<fn(&mut ShellCore, args: &mut Vec<String>) -> i32> {
        if self.builtins.contains_key(name) {
            Some(self.builtins[name])
        }else{
            None
        }
    }

    pub fn has_flag(&self, flag: char) -> bool {
        if let Some(_) = self.flags.find(flag) {
            return true;
        }
        false
    }

    pub fn wait_process(&mut self, child: Pid) {
        let exit_status = match waitpid(child, Some(WaitPidFlag::WUNTRACED)) {
            Ok(WaitStatus::Exited(_pid, status)) => {
                status
            },
            Ok(WaitStatus::Signaled(pid, signal, _coredump)) => {
                eprintln!("Pid: {:?}, Signal: {:?}", pid, signal);
                128+signal as i32 
            },
            Ok(WaitStatus::Stopped(pid, signal)) => {
                /*TODO: The shell must restart other jobs if they are not sleeping.*/
                self.jobs[0].status = "Stopped".to_string();
                self.jobs[0].id = self.jobs.len();
                self.jobs[0].mark = '+';
                self.jobs[0].async_pids.push(pid);
                print!("\n{}", self.jobs[0].status_string().clone());
                self.add_bg_job(self.jobs[0].clone());
                128+signal as i32 
            },
            Ok(unsupported) => {
                eprintln!("Error: {:?}", unsupported);
                1
            },
            Err(err) => {
                panic!("Error: {:?}", err);
            },
        };

        self.set_var("?", &exit_status.to_string());
    } 

    pub fn read_pipe(&mut self, pin: RawFd, pid: Pid) -> String {
        let mut ans = "".to_string();
        let mut ch = [0;1000];
    
        loop {
            while let Ok(n) = read(pin, &mut ch) {
                ans += &String::from_utf8(ch[..n].to_vec()).unwrap();
                match waitpid(pid, Some(WaitPidFlag::WNOHANG)).expect("Faild to wait child process.") {
                    WaitStatus::StillAlive => {
                        continue;
                    },
                    WaitStatus::Exited(_pid, status) => {
                        self.set_var("?", &status.to_string());
                        break;
                    },
                    WaitStatus::Signaled(pid, signal, _) => {
                        self.set_var("?", &(128+signal as i32).to_string());
                        eprintln!("Pid: {:?}, Signal: {:?}", pid, signal);
                        break;
                    },
                    _ => {
                        break;
                    },
                };
            }
            return ans;
        }
    }

    pub fn wait_job(&mut self, job_no: usize) {
        if self.jobs[job_no].status == "Done" {
            return;
        }

        let mut pipestatus = vec![];
        for p in self.jobs[job_no].pids.clone() {
            self.wait_process(p);
            pipestatus.push(self.get_var("?"));
        }
        self.set_var("PIPESTATUS", &pipestatus.join(" "));
        self.jobs[job_no].status = "Done".to_string();
    }

    pub fn check_async_process(pid: Pid) -> bool {
        match waitpid(pid, Some(WaitPidFlag::WNOHANG)).expect("Faild to wait child process.") {
            WaitStatus::StillAlive =>  false,
            _                      => true
        }
    }

    pub fn check_jobs(&mut self) {
        for j in 1..self.jobs.len() {
            if self.jobs[j].async_pids.len() != 0 {
                self.jobs[j].check_of_finish();
            }
        }

        for j in 1..self.jobs.len() {
            if self.jobs[j].status == "Done" {
                self.jobs[j].print_status();
            }
        }

        while self.jobs.len() > 1 {
            let job = self.jobs.pop().unwrap();

            if job.status != "Printed" {
                self.jobs.push(job);
                break;
            }
        }
    }

    pub fn add_bg_job(&mut self, added: Job) {
        if added.mark == '+' {
            for job in self.jobs.iter_mut() {
                job.mark = if job.mark == '+' {'-'}else{' '};
            }
        }

        self.jobs.push(added);
    }
}
