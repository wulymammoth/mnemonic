mod default_toml_config;
#[cfg(test)]
pub mod test_state;
use crate::err::CliErr;
use clap::ArgMatches;
use derive_builder::Builder;
use serde_derive::{Deserialize, Serialize};
use std::{fs, io::prelude::*};
#[cfg(test)]
use test_state::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct State {
    mnemonics: Vec<String>,
    directory: String,
    add: Add,
    edit: Edit,
    list: List,
    rm: Rm,
    show: Show,
    filesystem: FileSystem,
}

impl State {
    // getters
    pub fn add(&self) -> &Add {
        &self.add
    }
    pub fn edit(&self) -> &Edit {
        &self.edit
    }
    pub fn show(&self) -> &Show {
        &self.show
    }
    pub fn rm(&self) -> &Rm {
        &self.rm
    }
    pub fn mnemonics(&self) -> &Vec<String> {
        &self.mnemonics
    }
    pub fn directory(&self) -> &String {
        &self.directory
    }
    pub fn filesystem(&self) -> &FileSystem {
        &self.filesystem
    }

    // Constructors/setters
    #[cfg(test)]
    pub fn from_test_state(test_state: TestState) -> Self {
        Self {
            mnemonics: test_state.mnemonics,
            directory: test_state.directory,
            add: test_state.add,
            edit: test_state.edit,
            list: test_state.list,
            rm: test_state.rm,
            show: test_state.show,
            filesystem: test_state.filesystem,
        }
    }

    pub fn from_config_file() -> Result<Self, CliErr> {
        use default_toml_config;
        use directories::ProjectDirs;
        use std::{fs, io::Read};
        use toml_edit::{value, Document};

        let config_dir = ProjectDirs::from("", "", "mn")
            .ok_or_else(|| CliErr::LocateDirs)?
            .config_dir()
            .to_str()
            .ok_or_else(|| CliErr::ParseUnicode("the data directory for your system".to_string()))?
            .to_string();

        let config_file = format!("{}/mn_config.toml", config_dir).to_string();

        let mut config = String::new();

        let state: State = match fs::File::open(&config_file) {
            Ok(mut file) => {
                file.read_to_string(&mut config)?;
                config.push_str("[filesystem]\nmnemonic_files = []");
                toml::from_str(&config)?
            }
            Err(_) => {
                use std::env;

                config = default_toml_config::TOML.to_string();
                config.push_str("[filesystem]\nmnemonic_files = []");
                let mut state: State = toml::from_str(config.as_str())?;
                let directory = ProjectDirs::from("", "", "mn")
                    .ok_or_else(|| CliErr::LocateDirs)?
                    .data_local_dir()
                    .to_str()
                    .ok_or_else(|| {
                        CliErr::ParseUnicode("the data directory for your system".to_string())
                    })?
                    .to_string();
                let default_editor = if let Some(editor) = env::var_os("VISUAL") {
                    editor
                        .into_string()
                        .map_err(|_| CliErr::ParseUnicode("VISUAL".to_string()))?
                } else if let Some(editor) = env::var_os("EDITOR") {
                    editor
                        .into_string()
                        .map_err(|_| CliErr::ParseUnicode("EDITOR".to_string()))?
                } else {
                    "nano".to_string()
                };
                state.edit.editor = default_editor.clone();
                state.add.editor = default_editor.clone();
                state.directory = directory.clone();
                let mut state_with_comments = default_toml_config::TOML.parse::<Document>()?;
                state_with_comments["edit"]["editor"] = value(default_editor.clone());
                state_with_comments["add"]["editor"] = value(default_editor);
                state_with_comments["directory"] = value(directory.clone());
                fs::create_dir_all(&config_dir)?;
                let mut file = fs::File::create(&config_file)?;
                file.write_all(state_with_comments.to_string().as_bytes())?;

                state
            }
        };

        Ok(state)
    }

    pub fn and_from_clap_args(self, clap_args: ArgMatches) -> Self {
        Self {
            mnemonics: match clap_args.values_of("MNEMONIC") {
                Some(clap_vec) => clap_vec.map(|s| s.to_string()).collect(),
                None => match clap_args.subcommand() {
                    ("add", Some(sub_args))
                    | ("edit", Some(sub_args))
                    | ("rm", Some(sub_args))
                    | ("show", Some(sub_args)) => match sub_args.values_of("MNEMONIC") {
                        Some(clap_vec) => clap_vec.map(|s| s.to_string()).collect(),
                        None => self.mnemonics,
                    },
                    (_, _) => self.mnemonics,
                },
            },
            add: Add {
                blank: match clap_args.subcommand_matches("add") {
                    Some(add_args) => add_args.is_present("blank") || self.add.blank,
                    None => self.add.blank,
                },
                editor: match (
                    clap_args.subcommand_matches("add"),
                    clap_args.subcommand_matches("edit"),
                ) {
                    (Some(add_args), _) => match add_args.value_of("editor") {
                        Some(editor) => editor.to_string(),
                        None => self.add.editor.clone(),
                    },

                    (_, Some(edit_args)) => match edit_args.value_of("editor") {
                        Some(editor) => editor.to_string(),
                        None => self.add.editor.clone(),
                    },
                    (None, None) => self.add.editor.clone(),
                },
            },
            edit: Edit {
                push: match clap_args.subcommand_matches("edit") {
                    Some(edit_args) => edit_args.value_of("push").map(|s| s.to_string()),
                    None => self.edit.push,
                },
                editor: match (
                    clap_args.subcommand_matches("add"),
                    clap_args.subcommand_matches("edit"),
                ) {
                    (Some(add_args), _) => match add_args.value_of("editor") {
                        Some(editor) => editor.to_string(),
                        None => self.add.editor,
                    },

                    (_, Some(edit_args)) => match edit_args.value_of("editor") {
                        Some(editor) => editor.to_string(),
                        None => self.add.editor,
                    },
                    (None, None) => self.add.editor,
                },
            },
            rm: Rm {
                force: match clap_args.subcommand_matches("rm") {
                    Some(rm_args) => rm_args.is_present("force") || self.rm.force,
                    None => self.rm.force,
                },
            },
            show: Show {
                plaintext: match (clap_args.subcommand_matches("show"), &clap_args) {
                    (Some(show_arg), _) => show_arg.is_present("plaintext") || self.show.plaintext,
                    (_, clap_args) => clap_args.is_present("plaintext") || self.show.plaintext,
                },
                theme: match (clap_args.subcommand_matches("show"), &clap_args) {
                    (Some(show_args), _) => match show_args.value_of("theme") {
                        Some(theme) => theme.to_string(),
                        None => self.show.theme,
                    },
                    (_, clap_args) => match clap_args.value_of("theme") {
                        Some(theme) => theme.to_string(),
                        None => self.show.theme,
                    },
                },
                syntax: match (clap_args.subcommand_matches("show"), &clap_args) {
                    (Some(show_args), _) => match show_args.value_of("syntax") {
                        Some(syntax) => syntax.to_string(),
                        None => self.show.syntax,
                    },
                    (_, clap_args) => match clap_args.value_of("syntax") {
                        Some(syntax) => syntax.to_string(),
                        None => self.show.syntax,
                    },
                },
            },
            ..self
        }
    }

    pub fn and_from_filesystem(self) -> Result<Self, CliErr> {
        fs::create_dir_all(&self.directory)?;
        let dir_contents = fs::read_dir(&self.directory)?;

        let mut mnemonic_files = Vec::new();
        for file in dir_contents {
            mnemonic_files.push(
                file?
                    .path()
                    .file_stem()
                    .ok_or_else(|| CliErr::ParseUnicode("mnemonic file".to_string()))?
                    .to_str()
                    .ok_or_else(|| CliErr::ParseUnicode("mnemonic file".to_string()))?
                    .to_string(),
            );
        }
        Ok(Self {
            filesystem: FileSystem { mnemonic_files },
            ..self
        })
    }

    pub fn with_new_mnemonic_file(self, filename: String) -> Self {
        let mnemonic_files: Vec<String> = self
            .filesystem
            .mnemonic_files
            .clone()
            .into_iter()
            .chain(vec![filename])
            .collect();
        Self {
            filesystem: FileSystem { mnemonic_files },
            ..self
        }
    }
}

#[derive(Deserialize, Serialize, Builder, Debug, Clone, Default)]
#[builder(setter(into), default)]
pub struct Add {
    blank: bool,
    editor: String,
}

impl Add {
    pub fn blank(&self) -> &bool {
        &self.blank
    }

    pub fn editor(&self) -> &String {
        &self.editor
    }
}

#[derive(Deserialize, Serialize, Builder, Debug, Clone, Default)]
#[builder(setter(into), default)]
pub struct Edit {
    push: Option<String>,
    editor: String,
}

impl Edit {
    pub fn push(&self) -> &Option<String> {
        &self.push
    }
}

#[derive(Deserialize, Serialize, Builder, Debug, Clone, Default)]
#[builder(setter(into), default)]
pub struct List {}

#[derive(Deserialize, Serialize, Builder, Debug, Clone, Default)]
#[builder(setter(into), default)]
pub struct Rm {
    force: bool,
}

impl Rm {
    pub fn force(&self) -> &bool {
        &self.force
    }
}

#[derive(Deserialize, Serialize, Builder, Debug, Clone)]
#[builder(setter(into), default)]
pub struct Show {
    plaintext: bool,
    syntax: String,
    theme: String,
}

impl Show {
    pub fn syntax(&self) -> &String {
        &self.syntax
    }
    pub fn theme(&self) -> &String {
        &self.theme
    }
    pub fn plaintext(&self) -> &bool {
        &self.plaintext
    }
}

impl Default for Show {
    fn default() -> Self {
        Self {
            plaintext: false,
            syntax: "md".to_string(),
            theme: "TwoDark".to_string(),
        }
    }
}

#[derive(Deserialize, Serialize, Builder, Debug, Clone, Default)]
#[builder(setter(into), default)]
pub struct FileSystem {
    mnemonic_files: Vec<String>,
}

impl FileSystem {
    pub fn mnemonic_files(&self) -> &Vec<String> {
        &self.mnemonic_files
    }
}
