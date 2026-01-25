use anyhow::Result;
use async_imap::error::Error;
use async_imap::Session;
use async_native_tls::TlsStream;
use async_std::net::TcpStream;
use futures::TryStreamExt;
use crate::imap::variables::{get_id, get_imap_addr, get_imap_port, get_pw};

pub mod variables;

pub type IMAPSession = Session<TlsStream<TcpStream>>;

pub async fn create_client() -> Result<IMAPSession> {
    let tcp_stream = TcpStream::connect((get_imap_addr()?.as_str(), get_imap_port()?)).await?;
    let tls = async_native_tls::TlsConnector::new();
    let tls_stream = tls.connect(get_imap_addr()?.as_str(), tcp_stream).await?;

    let client = async_imap::Client::new(tls_stream);
    println!("Connected to imap server!");

    let session = client.login(get_id()?, get_pw()?).await.map_err(|e| e.0)?;
    println!("Logged in!");
    Ok(session)
}

pub async fn load_mailboxes(session: &mut IMAPSession) -> Result<()> {
    let mut mailboxes = session.list(None, Some("*")).await?;
    println!("Available mailboxes:");
    while let Some(mailbox) = mailboxes.try_next().await? {
        println!("mailbox {:?}", mailbox.name());
    }

    Ok(())
}

pub async fn create_mailbox_if_not_exists(session: &mut IMAPSession, mailbox: &str) -> Result<()> {
    match session.create(mailbox).await {
        Ok(_) => Ok(()),
        Err(Error::No(_)) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub trait MoveCapability {
    async fn move_mail(self: &mut Self, uid: String, destination: &str) -> Result<()>;
}

impl MoveCapability for IMAPSession {
    async fn move_mail(self: &mut Self, uid: String, destination: &str) -> Result<()> {
        println!("Moving {} to {}", uid, destination);
        create_mailbox_if_not_exists(self, &destination).await?;
        self.uid_copy(&uid, format!("\"{destination}\"")).await?;
        self.uid_store(&uid, "+FLAGS (\\Deleted)").await?;
        self.expunge().await?;

        Ok(())
    }
}
