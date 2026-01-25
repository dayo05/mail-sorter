use std::env;
use anyhow::Result;

pub fn get_id() -> Result<String> {
    Ok(env::var("MAIL_ID")?)
}

pub fn get_pw() -> Result<String> {
    Ok(env::var("MAIL_PW")?)
}

pub fn get_imap_addr() -> Result<String> {
    Ok(env::var("MAIL_IMAP_ADDR")?)
}

pub fn get_imap_port() -> Result<u16> {
    Ok(env::var("MAIL_IMAP_PORT").unwrap_or("993".to_string()).parse()?)
}

pub fn get_general_mail() -> Result<Vec<String>> {
    Ok(env::var("MAIL_GENERAL")?.split(",").map(|s| s.to_string()).collect())
}

pub fn get_mail_regex() -> Result<Vec<String>> {
    Ok(env::var("MAIL_REX")?.split(",").map(|s| s.to_string()).collect())
}