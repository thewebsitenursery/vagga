use std::collections::{BTreeMap, HashSet};

use config::{Config};
use config::command::{MainCommand, SuperviseInfo};
use config::containers::Container;


#[derive(PartialEq, Eq, Hash)]
struct CommandOption<'a> {
    names: &'a [&'a str],
    has_args: bool,
    single: bool,
}

struct BuiltinCommand<'a> {
    name: &'a str,
    accept_container: bool,
    options: &'a [&'a CommandOption<'a>],
}

#[derive(Clone)]
struct SuperviseCommand<'a> {
    name: &'a str,
    info: &'a SuperviseInfo,
    options: &'a [&'a SuperviseOption<'a>],
}

struct SuperviseOption<'a> {
    names: &'a [&'a str],
    has_args: bool,
    single: bool,
    accept_children: bool,
}

/**

Transition table:
               ________                     ______________
 +————————————|        |——————————————————>|              |
 |  +—————————| Global |                   | SuperviseCmd |
 |  |  +——————|________|<————————+   +—————|______________|<————+
 |  |  |                         |   |                          |
 |  |  |    ______________       |   |     _________________    |?
 |  |  |   |              |      |   +———>|                 |———+
 |  |  +——>| GlobalOption |——————+        | SuperviseOption |
 |  |      |______________|          +————|_________________|<————+
 |  |                                |                            |
 |  |         _________              |    ____________________    |
 |  +———————>|         |             |   |                    |   |
 |  +————————| Command |<——————+     +——>| SuperviseOptionArg |<——+
 |  |  +—————|_________|       |         |____________________|
 |  |  |                       |
 |  |  |    _______________    |
 |  |  |   |               |   |
 |  |  +——>| CommandOption |———+
 |  |      |_______________|
 |  |
 |  |        _____________
 |  +——————>|             |
 |          | CommandArgs |
 +—————————>|_____________|

*/
enum States<'a> {
    Global,
    GlobalOption(&'a CommandOption<'a>),
    Command(&'a BuiltinCommand<'a>),
    CommandOption(&'a BuiltinCommand<'a>, &'a CommandOption<'a>),
    CommandArgs,
    SuperviseCmd(SuperviseCommand<'a>),
    SuperviseOption(SuperviseCommand<'a>, &'a SuperviseOption<'a>),
    SuperviseOptionArg(SuperviseCommand<'a>, &'a SuperviseOption<'a>),
}

struct CompletionState<'a> {
    commands: &'a BTreeMap<String, MainCommand>,
    containers: &'a BTreeMap<String, Container>,
    state: States<'a>,
    single_global_options: HashSet<&'a CommandOption<'a>>,
    single_command_options: HashSet<&'a CommandOption<'a>>,
    supervise_chosen_children: HashSet<&'a str>,
}

impl<'a> CompletionState<'a> {
    pub fn new(
        commands: &'a BTreeMap<String, MainCommand>,
        containers: &'a BTreeMap<String, Container>
    ) -> CompletionState<'a> {

        CompletionState {
            commands: commands,
            containers: containers,
            state: States::Global,
            single_global_options: HashSet::new(),
            single_command_options: HashSet::new(),
            supervise_chosen_children: HashSet::new(),
        }
    }

    pub fn trans(&mut self, arg: &'a str) {
        let mut next_state: Option<States> = None;
        match self.state {
            States::Global => {
                for cmd in BUILTIN_COMMANDS {
                    if cmd.name == arg {
                        self.state = States::Command(cmd);
                        return;
                    }
                }
                for opt in GLOBAL_OPTIONS {
                    for &opt_name in opt.names {
                        if arg == opt_name {
                            if opt.has_args {
                                self.state = States::GlobalOption(opt);
                            }
                            if opt.single {
                                self.single_global_options.insert(opt);
                            }
                            return;
                        }
                    }
                }
                for (cmd_name, user_cmd) in self.commands.iter() {
                    if arg != cmd_name {
                        continue;
                    }
                    match *user_cmd {
                        MainCommand::Command(_) => {
                            self.state = States::CommandArgs;
                            return;
                        },
                        MainCommand::Supervise(ref supervise_info) => {
                            let supervise_cmd = SuperviseCommand {
                                name: cmd_name,
                                info: supervise_info,
                                options: SUPERVISE_OPTIONS,
                            };
                            self.state = States::SuperviseCmd(supervise_cmd);
                            return;
                        },
                    }
                }
                self.state = States::CommandArgs;
            },
            States::GlobalOption(_) => {
                self.state = States::Global;
            },
            States::Command(cmd) => {
                for cmd_opt in cmd.options {
                    for &opt_name in cmd_opt.names {
                        if arg == opt_name {
                            if cmd_opt.has_args {
                                self.state = States::CommandOption(cmd, cmd_opt);
                            }
                            if cmd_opt.single {
                                self.single_command_options.insert(cmd_opt);
                            }
                            return;
                        }
                    }
                }
                self.state = States::CommandArgs;
            },
            States::CommandOption(cmd, _) => {
                self.state = States::Command(cmd);
            },
            States::CommandArgs => {},
            States::SuperviseCmd(ref cmd) => {
                'options_label: for cmd_opt in cmd.options {
                    for &opt_name in cmd_opt.names {
                        if arg == opt_name {
                            next_state = Some(States::SuperviseOption(cmd.clone(), cmd_opt));
                            break 'options_label;
                        }
                    }
                }
            },
            States::SuperviseOption(ref cmd, opt) => {
                if cmd.info.children.contains_key(arg) {
                    self.supervise_chosen_children.insert(arg);
                } else {
                    next_state = Some(States::SuperviseCmd(cmd.clone()));
                }
            },
            States::SuperviseOptionArg(ref cmd, opt) => {
            },
        }

        if let Some(next_state) = next_state {
            self.state = next_state;
        }
    }
    
    pub fn complete(&self, cur: &str) -> Vec<&str> {
        let mut completions: Vec<&str> = Vec::new();
        match self.state {
            States::Global => {
                completions.extend(self.commands.keys().map(|c| &c[..]));
                if cur.starts_with("_") {
                    completions.extend(BUILTIN_COMMANDS.iter().map(|c| c.name));
                }
                if cur.starts_with("-") {
                    for opt in GLOBAL_OPTIONS {
                        if !self.single_global_options.contains(opt) {
                            completions.extend(opt.names);
                        }
                    }
                }
            },
            States::Command(cmd) => {
                if cmd.accept_container {
                    completions.extend(self.containers.keys().map(|c| &c[..]));
                }
                if cur.starts_with("-") {
                    for opt in cmd.options {
                        if !self.single_command_options.contains(opt) {
                            completions.extend(opt.names);
                        }
                    }
                }
            },
            States::SuperviseCmd(ref supervise_cmd) => {
                for opt in supervise_cmd.options {
                    completions.extend(opt.names);
                }
            },
            States::SuperviseOption(ref supervise_cmd, supervise_opt) => {
                // TODO: specify which supervise options can accept child as argument
                for child in supervise_cmd.info.children.keys() {
                    let child_name = &child[..];
                    if !self.supervise_chosen_children.contains(child_name) {
                        completions.push(child_name);
                    }
                }
            },
            _ => {},
        }
        completions.retain(|c| c.starts_with(cur));
        return completions;
    }
}

const BUILTIN_COMMANDS: &'static [&'static BuiltinCommand<'static>] = &[
    &BuiltinCommand { 
        name: "_build",
        accept_container: true,
        options: &[
            &CommandOption { names: &["--force"], has_args: false, single: true },
        ]
    },
    &BuiltinCommand { 
        name: "_build_shell",
        accept_container: false,
        options: &[]
    },
    &BuiltinCommand { 
        name: "_clean",
        accept_container: false,
        options: &[
            &CommandOption { names: &["--tmp", "--tmp-folders"], has_args: false, single: true },
            &CommandOption { names: &["--old", "--old-containers"], has_args: false, single: true },
            &CommandOption { names: &["--unused"], has_args: false, single: true },
            &CommandOption { names: &["--transient"], has_args: false, single: true },
            &CommandOption { names: &["--global"], has_args: false, single: true },
            &CommandOption { names: &["-n", "--dry-run"], has_args: false, single: true },
        ]
    },
    &BuiltinCommand { 
        name: "_create_netns",
        accept_container: false,
        options: &[
            &CommandOption { names: &["--dry-run"], has_args: false, single: true },
            &CommandOption { names: &["--no-iptables"], has_args: false, single: true },
        ]
    },
    &BuiltinCommand { 
        name: "_destroy_netns",
        accept_container: false,
        options: &[
            &CommandOption { names: &["--dry-run"], has_args: false, single: true },
            &CommandOption { names: &["--no-iptables"], has_args: false, single: true },
        ]
    },
    &BuiltinCommand { 
        name: "_init_storage_dir",
        accept_container: false,
        options: &[]
    },
    &BuiltinCommand { 
        name: "_list",
        accept_container: false,
        options: &[]
    },
    &BuiltinCommand { 
        name: "_pack_image",
        accept_container: true,
        options: &[
            &CommandOption { names: &["-f", "--file"], has_args: true, single: true },
        ]
    },
    &BuiltinCommand { 
        name: "_run",
        accept_container: true,
        options: &[
            &CommandOption { names: &["-W", "--writable"], has_args: false, single: true },
        ]
    },
    &BuiltinCommand { 
        name: "_run_in_netns",
        accept_container: true,
        options: &[
            &CommandOption { names: &["--pid"], has_args: true, single: true },
        ]
    },
    &BuiltinCommand { 
        name: "_version_hash",
        accept_container: true,
        options: &[
            &CommandOption { names: &["-s", "--short"], has_args: false, single: true },
            &CommandOption { names: &["-fd3"], has_args: false, single: true },
        ]
    },
];

const GLOBAL_OPTIONS: &'static [&'static CommandOption<'static>] = &[
    &CommandOption { names: &["-E", "--env", "--environ"], has_args: true, single: false },
    &CommandOption { names: &["-e", "--use-env"], has_args: true, single: false },
    &CommandOption { names: &["--ignore-owner-check"], has_args: false, single: true },
    &CommandOption { names: &["--no-build"], has_args: false, single: true },
    &CommandOption { names: &["--no-version-check"], has_args: false, single: true },
];

const SUPERVISE_OPTIONS: &'static [&'static SuperviseOption<'static>] = &[
    &SuperviseOption { names: &["--only"], has_args: true, single: false, accept_children: true },
    &SuperviseOption { names: &["--exclude"], has_args: true, single: false, accept_children: true },
];


pub fn generate_completions(config: &Config, args: Vec<String>) -> Result<i32, String> {
    let default_cur_arg = "".to_string();
    let mut splitted_args = args.splitn(2, |a| a == "--");
    let full_args = match splitted_args.next() {
        Some(a) => a.iter().collect::<Vec<_>>(),
        None => vec!(),
    };
    let cur_arg = match splitted_args.next() {
        Some(a) => a.get(0).unwrap_or(&default_cur_arg),
        None => &default_cur_arg,
    };

    let mut state = CompletionState::new(&config.commands, &config.containers);
    for arg in full_args {
        state.trans(arg);
    }
    for comp in state.complete(cur_arg) {
        println!("{}", comp);
    }

    Ok(0)
}
