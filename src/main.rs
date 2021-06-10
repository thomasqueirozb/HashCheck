mod db;
mod error;

use anyhow::Result;
use db::HashCheck;

use serde_derive::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Clone)]
struct Config {
    search: Search,

    #[cfg(feature = "email")]
    email: Email,

    #[cfg(feature = "email_sparkpost")]
    sparkpost: Sparkpost,
}

#[derive(Deserialize, Clone)]
struct Search {
    include: Vec<PathBuf>,
    exclude: Option<Vec<PathBuf>>,
}

#[cfg(feature = "email")]
#[derive(Deserialize, Clone)]
struct Email {
    sender: String,
    sender_name: String,
    recipient: String,
}

#[cfg(feature = "email_sparkpost")]
#[derive(Deserialize, Clone)]
struct Sparkpost {
    key: String,
}

fn main() {
    if let Err(e) = master() {
        eprintln!("{}", e);
    }
}

fn master() -> Result<()> {
    let config: Config = match toml::from_str(&fs::read_to_string("/etc/hash_check.toml")?) {
        Ok(c) => Ok(c),
        Err(e) => {
            eprint!("TOML parse error: ");
            Err(e)
        }
    }?;

    if let Err(e) = check_hashes(config.clone()) {
        #[cfg(feature = "email")]
        {
            let subject = "Hash Checker fail";
            #[cfg(feature = "email_sparkpost")]
            {
                use sparkpost::transmission::{
                    EmailAddress, Message, Transmission, TransmissionResponse,
                };
                let tm = Transmission::new(config.sparkpost.key);
                let mut email: Message = Message::new(EmailAddress::new(
                    config.email.sender,
                    config.email.sender_name,
                ));

                email
                    .add_recipient(config.email.recipient)
                    .subject(subject)
                    // .html("<h1>html body of the email</h1>")
                    .text(e.to_string());

                match tm.send(&email) {
                    Err(email_err) => eprintln!("Could not send sparkpost email: {}", email_err),
                    Ok(res) => {
                        println!("{:?}", &res);
                        match res {
                            TransmissionResponse::ApiResponse(api_res) => {
                                let rec = api_res.total_accepted_recipients;
                                if rec != 1 {
                                    eprintln!("Sparkpost accepeted recipents: {}", rec);
                                }
                                let rej = api_res.total_rejected_recipients;
                                if rej != 0 {
                                    eprintln!("Sparkpost rejected recipents: {}", rec);
                                }
                            }
                            TransmissionResponse::ApiError(errors) => {
                                eprintln!("Sparkpost Error: \n {:#?}", &errors);
                            }
                        }
                    }
                }
            }
            #[cfg(feature = "email_sendmail")]
            {
                if let Err(sendmail_err) = sendmail::email::send(
                    &config.email.sender,
                    &[config.email.recipient.as_str()],
                    subject,
                    e.to_string().as_str(),
                ) {
                    eprintln!("Sendmail error: {}", sendmail_err);
                }
            }
        }
        return Err(e);
    }

    Ok(())
}

fn check_hashes(config: Config) -> Result<()> {
    // .with_context(|e| format!("Failed to parse TOML: {}", e))?;

    // println!("{:?}", config.search.exclude.unwrap());
    let exclude: HashSet<PathBuf> = {
        if let Some(ex) = config.search.exclude {
            ex.iter().cloned().collect()
        } else {
            HashSet::new()
        }
    };

    let mut paths: VecDeque<PathBuf> = config.search.include.iter().cloned().collect();

    let (new, conn) = db::open("/var/lib/hash_check/hash_check.sqlite")?;

    let check = |file: PathBuf| -> Result<()> {
        let mut hasher = Sha256::new();
        let file_string = file.display().to_string();
        hasher.update(fs::read(file)?);
        let result = hasher.finalize();

        if new {
            conn.insert(file_string, result)?;
        } else {
            conn.compare(file_string, result)?;
        }

        Ok(())
    };

    while let Some(p) = paths.pop_back() {
        if exclude.contains(&p) {
            println!("Excluded {}", p.clone().display());
            continue;
        }

        if !p.exists() {
            eprintln!("Path \"{}\" does not exist.", p.display());
            continue;
        }

        let ft = fs::metadata(p.clone())?.file_type();

        #[cfg(unix)]
        if ft.is_symlink() {
            continue;
        }

        if ft.is_file() {
            check(p)?;
        } else {
            if let Ok(entries) = fs::read_dir(p.as_path()) {
                for entry in entries {
                    let entry = entry?;
                    if exclude.contains(&entry.path()) {
                        continue;
                    }

                    let ft = entry.file_type()?;

                    #[cfg(unix)]
                    if ft.is_symlink() {
                        continue;
                    }

                    if ft.is_file() {
                        check(entry.path())?;
                    } else {
                        paths.push_front(entry.path());
                    }
                }
            } else {
                eprintln!("Could not open {:#?}", p);
            }
        }
    }

    Ok(())
}
