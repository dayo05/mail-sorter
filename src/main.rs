use crate::imap::variables::{get_general_mail, get_mail_regex};
use crate::imap::*;
use anyhow::Result;
use async_imap::types::Uid;
use futures::StreamExt;
use mailparse::MailAddr::{Group, Single};
use mailparse::{SingleInfo, addrparse, parse_headers};
use regex::Regex;
use std::cmp::max;

mod imap;

async fn process_mail(session: &mut IMAPSession, header: BasicMailHeader) -> Result<()> {
    println!("Processing mail: {:?}", header);

    if header.from.ends_with("@uos.ac.kr") {
        session.move_mail(header.uid.to_string(), "UOS").await?;
        return Ok(());
    }
    if header.from.ends_with("@linkedin.com") {
        session
            .move_mail(header.uid.to_string(), "Special/Linked in")
            .await?;
        return Ok(());
    }
    if header.from.ends_with("@inflearn.com") {
        session
            .move_mail(header.uid.to_string(), "Special/Inflearn")
            .await?;
        return Ok(());
    }
    if header.from.ends_with("@accounts.google.com") {
        session
            .move_mail(header.uid.to_string(), "Special/Google")
            .await?;
        return Ok(());
    }
    if header.from.ends_with("@x.com") {
        session
            .move_mail(header.uid.to_string(), "Special/X")
            .await?;
        return Ok(());
    }

    if !header.to.is_empty() {
        let header_to_acc = header.to.split_once("@").unwrap().0;
        if !get_general_mail()?.contains(&header_to_acc.to_string()) {
            session
                .move_mail(
                    header.uid.to_string(),
                    format!("Special/{header_to_acc}").as_str(),
                )
                .await?;
            return Ok(());
        }
    }
    else {
        session.move_mail(header.uid.to_string(), "Cc").await?;
    }

    Ok(())
}

async fn apply_to_all(session: &mut IMAPSession) -> Result<()> {
    session.select("INBOX").await?;
    process_bulk(session, "1:*", true, 1).await?;
    Ok(())
}

async fn process_bulk(
    session: &mut IMAPSession,
    range: &str,
    force_apply: bool,
    mut next_expected_uid: u32,
) -> Result<u32> {
    let mut to_be_processed = vec![];
    {
        let mut fetch_stream = session.uid_fetch(range, "(UID RFC822.HEADER)").await?;

        while let Some(Ok(msg)) = fetch_stream.next().await {
            if let Some(uid) = msg.uid {
                if force_apply || uid >= next_expected_uid {
                    let header = msg.header().unwrap();
                    let uid = msg.uid.unwrap();

                    let (headers, _) = parse_headers(header).unwrap();
                    let get_val = |key: &str| {
                        headers
                            .iter()
                            .find(|h| h.get_key().to_lowercase() == key.to_lowercase())
                            .map(|h| h.get_value())
                    };

                    let from = get_val("From").unwrap();
                    let subject = get_val("Subject").unwrap();
                    let date = get_val("Date").unwrap();
                    let to = get_val("To").unwrap();
                    let cc = get_val("Cc").unwrap_or("".to_string());
                    let bcc = get_val("Bcc").unwrap_or("".to_string());

                    let from_addr_list = addrparse(from.as_str())?
                        .extract_single_info()
                        .expect(format!("From parse failed: {from}").as_str());

                    let to_addr_list = match addrparse(to.as_str()) {
                        Ok(x) => x.iter()
                            .find_map(|x| match x {
                                Group(g) => {
                                    for x in get_mail_regex().unwrap() {
                                        let regex = Regex::new(x.as_str()).unwrap();
                                        for i in g.addrs.iter() {
                                            println!("Match: {i}");
                                            if regex.is_match(i.addr.as_str()) {
                                                return Some(i.clone());
                                            }
                                        }
                                    }
                                    None
                                }
                                Single(s) => {
                                    for x in get_mail_regex().unwrap() {
                                        let regex = Regex::new(x.as_str()).unwrap();
                                        if regex.is_match(s.addr.as_str()) {
                                            return Some(s.clone());
                                        }
                                    }
                                    None
                                }
                            })
                            .unwrap_or(SingleInfo {
                                display_name: None,
                                addr: "".to_string(),
                            }),
                        Err(e) => SingleInfo {
                            display_name: None,
                            addr: "".to_string(),
                        }
                    };

                    let basic_header = BasicMailHeader {
                        uid,
                        from: from_addr_list.addr.clone(),
                        from_display_name: match from_addr_list.display_name {
                            Some(x) => x,
                            _ => from_addr_list.addr,
                        },
                        to: to_addr_list.addr.clone(),
                        to_display_name: match to_addr_list.display_name {
                            Some(x) => x,
                            _ => to_addr_list.addr,
                        },
                        subject,
                        date,
                    };

                    to_be_processed.push(basic_header);

                    next_expected_uid = max(next_expected_uid, uid + 1);
                }
            }
        }
    }

    for header in to_be_processed {
        process_mail(session, header).await?;
    }

    Ok(next_expected_uid)
}

#[derive(Debug)]
struct BasicMailHeader {
    uid: Uid,
    from: String,
    from_display_name: String,
    to: String,
    to_display_name: String,
    subject: String,
    date: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut session = create_client().await?;

    let capability = session.capabilities().await?;
    for x in capability.iter() {
        println!("{:?}", x);
    }

    load_mailboxes(&mut session).await?;
    let inbox = session.select("INBOX").await?;
    let mut next_expected_uid = 0;
    let mut current_uid_validity = inbox.uid_validity.unwrap();

    apply_to_all(&mut session).await?;

    loop {
        let mut idle_session = session.idle();
        idle_session.init().await?;
        let (idle_wait, _) = idle_session.wait();
        idle_wait.await?;

        session = idle_session.done().await?;

        let uid_validity = session.select("INBOX").await?.uid_validity.unwrap();
        if current_uid_validity != uid_validity {
            current_uid_validity = uid_validity;
            next_expected_uid = 0;
        }

        next_expected_uid = process_bulk(
            &mut session,
            format!("{}:*", next_expected_uid).as_str(),
            false,
            next_expected_uid,
        )
            .await?;
    }
}
