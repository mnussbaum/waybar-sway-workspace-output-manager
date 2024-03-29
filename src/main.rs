#[macro_use]
extern crate serde_derive;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::prelude::*;

use failure::Error;

use i3ipc::event::Event;
use i3ipc::reply;
use i3ipc::I3Connection;
use i3ipc::I3EventListener;
use i3ipc::Subscription;

const PACKAGE_NAME: &'static str = env!("CARGO_PKG_NAME");

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    background_colors: Vec<String>,
    focused_foreground_color: String,

    #[serde(default = "default_minimum_workspace_count")]
    minimum_workspace_count: usize,

    version: String,
}

fn default_minimum_workspace_count() -> usize {
    return 0;
}

fn workspace_module_output(
    config: &Config,
    workspace: &reply::Workspace,
    last_workspace_num: Option<i32>,
    workspace_count: i32,
) -> String {
    let mut module_text = workspace.num.to_string();
    if workspace.focused {
        module_text = format!(
            "<span color=\"{}\">{}</span>",
            config.focused_foreground_color, module_text,
        );
    }

    let color =
        &config.background_colors[(workspace.num - 1) as usize % config.background_colors.len()];
    let left_color = if let Some(last_workspace_num) = last_workspace_num {
        &config.background_colors
            [(last_workspace_num - 1) as usize % config.background_colors.len()]
    } else {
        color
    };

    if workspace.num >= workspace_count && workspace_count == 1 {
        return format!(
            "<span background=\"{}\"> {} </span><span size=\"large\" color=\"{}\"></span>\n",
            color, module_text, color,
        );
    } else if workspace.num == 1 {
        return format!(
            "<span size=\"large\" background=\"{}\"> {}</span>\n",
            color, module_text,
        );
    } else if workspace.num >= workspace_count {
        return format!(
            "<span size=\"large\" background=\"{}\" color=\"{}\"></span><span size=\"large\" background=\"{}\" color=\"{}\"></span><span background=\"{}\"> {} </span><span size=\"large\" color=\"{}\"></span>\n",
            left_color,
            left_color,
            left_color,
            color,
            color,
            module_text,
            color,
        );
    } else {
        return format!(
            "<span size=\"large\" background=\"{}\" color=\"{}\"></span><span size=\"large\" background=\"{}\" color=\"{}\"></span><span background=\"{}\"> {}</span>\n",
            left_color,
            left_color,
            left_color,
            color,
            color,
            module_text,
        );
    }
}

fn refresh_workspaces(
    config: &Config,
    workspaces: Vec<reply::Workspace>,
    output_dir: &std::path::Path,
    workspace_module_outputs: &mut HashMap<i32, String>,
) -> Result<(), Error> {
    let workspace_count = workspaces.len() as i32;

    let mut latest_workspace_nums: HashSet<i32> = HashSet::new();
    let mut last_workspace_num: Option<i32> = None;
    for workspace in workspaces {
        let module_output =
            workspace_module_output(config, &workspace, last_workspace_num, workspace_count);
        let old_module_output =
            if let Some(old_module_output) = workspace_module_outputs.get(&workspace.num) {
                old_module_output
            } else {
                ""
            };

        if module_output != *old_module_output {
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(output_dir.join(workspace.num.to_string()))?
                .write_all(module_output.as_bytes())?;
        }
        workspace_module_outputs.insert(workspace.num, module_output);
        latest_workspace_nums.insert(workspace.num);
        last_workspace_num = Some(workspace.num);
    }

    for existing_workspace_output in output_dir.read_dir()? {
        let workspace_output_entry = existing_workspace_output?.file_name().into_string();
        let workspace_num: i32 = match workspace_output_entry {
            Ok(s) => s.parse()?,
            Err(_) => {
                fs::remove_dir_all(output_dir)?;
                return Ok(());
            }
        };
        if latest_workspace_nums.contains(&workspace_num) {
            continue;
        }

        fs::OpenOptions::new()
            .append(true)
            .open(output_dir.join(workspace_num.to_string()))?
            .write_all("\n\n".as_bytes())?;
        workspace_module_outputs.remove(&workspace_num);
    }

    return Ok(());
}

#[derive(Debug)]
struct WorkspaceOutputManager<'a> {
    config: Config,
    wm_connection: RefCell<I3Connection>,
    wm_event_listener: RefCell<I3EventListener>,
    workspace_module_outputs: HashMap<i32, String>,
    output_dir: &'a std::path::Path,
}

impl<'a> WorkspaceOutputManager<'a> {
    fn new(
        output_dir: &'a std::path::Path,
        config: Config,
    ) -> Result<WorkspaceOutputManager<'a>, Error> {
        return Ok(WorkspaceOutputManager {
            config: config,
            output_dir: output_dir,
            wm_connection: RefCell::new(I3Connection::connect()?),
            wm_event_listener: RefCell::new(I3EventListener::connect()?),
            workspace_module_outputs: HashMap::new(),
        });
    }

    fn run(mut self) -> Result<(), Error> {
        if self.output_dir.exists() {
            fs::remove_dir_all(self.output_dir)?;
        }
        fs::create_dir_all(self.output_dir)?;
        let mut wm_event_listener = self.wm_event_listener.borrow_mut();
        wm_event_listener.subscribe(&[Subscription::Workspace])?;

        refresh_workspaces(
            &self.config,
            self.get_workspaces()?,
            self.output_dir,
            &mut self.workspace_module_outputs,
        )?;

        for event in wm_event_listener.listen() {
            match event? {
                Event::WorkspaceEvent(_) => {
                    refresh_workspaces(
                        &self.config,
                        self.get_workspaces()?,
                        self.output_dir,
                        &mut self.workspace_module_outputs,
                    )?;
                }
                _ => unreachable!(),
            }
        }

        return Ok(());
    }

    fn get_workspaces(&self) -> Result<Vec<reply::Workspace>, Error> {
        let mut presented_workspaces_by_num: HashMap<i32, reply::Workspace> = HashMap::new();
        let actual_workspaces = self.wm_connection.borrow_mut().get_workspaces()?.workspaces;
        let mut actual_workspaces_by_num: HashMap<i32, reply::Workspace> = HashMap::new();
        for workspace in actual_workspaces {
            actual_workspaces_by_num.insert(workspace.num, workspace);
        }

        for workspace_num in 1..=self.config.minimum_workspace_count {
            match actual_workspaces_by_num.remove(&(workspace_num as i32)) {
                None => presented_workspaces_by_num.insert(
                    workspace_num as i32,
                    reply::Workspace {
                        num: workspace_num as i32,
                        name: "".to_string(),
                        visible: false,
                        focused: false,
                        urgent: false,
                        rect: (0, 0, 0, 0),
                        output: "".to_string(),
                    },
                ),
                Some(workspace) => {
                    presented_workspaces_by_num.insert(workspace.num as i32, workspace)
                }
            };
        }

        for (_, workspace) in actual_workspaces_by_num {
            presented_workspaces_by_num.insert(workspace.num, workspace);
        }

        let mut presented_workspaces = Vec::new();
        for (_, workspace) in presented_workspaces_by_num.into_iter() {
            presented_workspaces.push(workspace);
        }

        presented_workspaces.sort_by(|a, b| a.num.cmp(&b.num));
        return Ok(presented_workspaces);
    }
}

fn main() {
    let output_dir = if let Some(cache_dir) = dirs::cache_dir() {
        cache_dir.join(PACKAGE_NAME)
    } else {
        eprintln!("No cache dir available for output files");
        std::process::exit(1);
    };

    let config_path = if let Some(config_dir) = dirs::config_dir() {
        config_dir.join(PACKAGE_NAME).join("config")
    } else {
        eprintln!("No config file available");
        std::process::exit(1);
    };

    let mut config_file = fs::File::open(config_path).unwrap();
    let mut config_contents = String::new();
    config_file.read_to_string(&mut config_contents).unwrap();
    let config: Config = serde_yaml::from_str(&config_contents).unwrap();

    let workspace_output_manager: WorkspaceOutputManager =
        match WorkspaceOutputManager::new(&output_dir, config) {
            Ok(workspace_output_manager) => workspace_output_manager,
            Err(e) => {
                eprintln!("Error instantiating workspace output manager: {}", e);
                std::process::exit(1);
            }
        };

    if let Err(e) = workspace_output_manager.run() {
        eprintln!("Error running workspace output manager: {}", e);
        std::process::exit(1);
    };
}
