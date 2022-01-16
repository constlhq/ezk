use super::timer::SessionTimer;
use super::Inner;
use crate::dialog::{Dialog, UsageGuard};
use crate::invite::AwaitedAck;
use sip_core::transaction::{ServerInvTsx, ServerTsx, TsxResponse};
use sip_core::transport::OutgoingResponse;
use sip_core::{Endpoint, IncomingRequest, Result};
use sip_types::header::typed::Refresher;
use sip_types::{Code, CodeKind, Method};
use std::sync::Arc;
use tokio::select;
use tokio::sync::mpsc::{self, Receiver};
use tokio::sync::oneshot;

#[derive(Debug, Clone, Copy)]
pub enum Role {
    Uac,
    Uas,
}

#[derive(Debug)]
pub struct Session {
    pub endpoint: Endpoint,
    inner: Arc<Inner>,

    pub role: Role,

    /// Receiver side of dialog-usage events
    usage_events: Receiver<UsageEvent>,

    session_timer: SessionTimer,

    // drop usage before dialog
    _usage_guard: UsageGuard,
    pub dialog: Arc<Dialog>,
}

pub struct RefreshNeeded<'s> {
    pub session: &'s mut Session,
}

impl RefreshNeeded<'_> {
    pub async fn process_default(self) -> Result<()> {
        let invite = self.session.dialog.create_request(Method::INVITE);

        let mut target_tp_info = self.session.dialog.target_tp_info.lock().await;

        let mut transaction = self
            .session
            .endpoint
            .send_invite(invite, &mut target_tp_info)
            .await?;

        drop(target_tp_info);

        let mut ack = None;

        while let Some(response) = transaction.receive().await? {
            match response.line.code.kind() {
                CodeKind::Provisional => { /* ignore */ }
                CodeKind::Success => {
                    let ack = if let Some(ack) = &mut ack {
                        ack
                    } else {
                        let ack_req = super::create_ack(
                            &self.session.dialog,
                            response.base_headers.cseq.cseq,
                        )
                        .await?;

                        ack.insert(ack_req)
                    };

                    self.session.endpoint.send_outgoing_request(ack).await?;
                }
                _ => { /* TODO: how to correctly handle responses here */ }
            }
        }

        Ok(())
    }
}

pub struct ReInviteReceived<'s> {
    pub session: &'s mut Session,
    pub invite: IncomingRequest,
    pub transaction: ServerInvTsx,
}

impl ReInviteReceived<'_> {
    /// Respond with a successful response, returns the received ACK request
    pub async fn respond_success(self, response: OutgoingResponse) -> Result<IncomingRequest> {
        let (ack_sender, ack_recv) = oneshot::channel();

        *self.session.inner.awaited_ack.lock() = Some(AwaitedAck {
            cseq: self.invite.base_headers.cseq.cseq,
            ack_sender,
        });

        let accepted = self.transaction.respond_success(response).await?;

        super::receive_ack(accepted, ack_recv).await
    }
}

pub struct ByeEvent<'s> {
    pub session: &'s mut Session,
    pub bye: IncomingRequest,
    pub transaction: ServerTsx,
}

impl ByeEvent<'_> {
    /// Process the BYE as one would expect, respond with a 200 OK
    pub async fn process_default(self) -> Result<()> {
        let response = self
            .session
            .dialog
            .create_response(&self.bye, Code::OK, None)?;

        self.transaction.respond(response).await
    }
}

#[allow(clippy::large_enum_variant)] // TODO address this
pub enum Event<'s> {
    RefreshNeeded(RefreshNeeded<'s>),
    ReInviteReceived(ReInviteReceived<'s>),
    Bye(ByeEvent<'s>),
    Terminated,
}

impl Session {
    pub(super) fn new(
        endpoint: Endpoint,
        inner: Arc<Inner>,
        role: Role,
        usage_events: mpsc::Receiver<UsageEvent>,
        session_timer: SessionTimer,
        usage_guard: UsageGuard,
        dialog: Dialog,
    ) -> Self {
        Self {
            endpoint,
            inner,
            role,
            usage_events,
            session_timer,
            _usage_guard: usage_guard,
            dialog: Arc::new(dialog),
        }
    }

    pub async fn drive(&mut self) -> Result<Event<'_>> {
        select! {
            _ = self.session_timer.wait() => {
               self.handle_session_timer().await
            }
            event = self.usage_events.recv() => {
                self.handle_usage_event(event)
            }
        }
    }

    pub async fn terminate(&mut self) -> Result<TsxResponse> {
        let mut state = self.inner.state.lock().await;
        state.set_terminated();

        let request = self.dialog.create_request(Method::BYE);

        let mut target_tp_info = self.dialog.target_tp_info.lock().await;

        let mut transaction = self
            .endpoint
            .send_request(request, &mut target_tp_info)
            .await?;

        drop(target_tp_info);

        transaction.receive_final().await
    }

    fn handle_usage_event(&mut self, evt: Option<UsageEvent>) -> Result<Event<'_>> {
        let evt = if let Some(evt) = evt {
            evt
        } else {
            // Usage events channel has been dropped,
            // because the state was set to Terminated.
            return Ok(Event::Terminated);
        };

        match evt {
            UsageEvent::Bye(request) => {
                let transaction = self.endpoint.create_server_tsx(&request);

                Ok(Event::Bye(ByeEvent {
                    session: self,
                    bye: request,
                    transaction,
                }))
            }
            UsageEvent::ReInvite(invite) => {
                self.session_timer.reset();

                let transaction = self.endpoint.create_server_inv_tsx(&invite);

                Ok(Event::ReInviteReceived(ReInviteReceived {
                    session: self,
                    invite,
                    transaction,
                }))
            }
        }
    }

    async fn handle_session_timer(&mut self) -> Result<Event<'_>> {
        match (self.role, self.session_timer.refresher) {
            (_, Refresher::Unspecified) => unreachable!(),
            (Role::Uac, Refresher::Uac) | (Role::Uas, Refresher::Uas) => {
                // We are responsible for the refresh
                // Timer expired meaning we are responsible for refresh now
                self.session_timer.reset();

                Ok(Event::RefreshNeeded(RefreshNeeded { session: self }))
            }
            (Role::Uac, Refresher::Uas) | (Role::Uas, Refresher::Uac) => {
                // Peer is responsible for refresh
                // Timer expired meaning we didn't get a RE-INVITE
                self.terminate().await?;
                Ok(Event::Terminated)
            }
        }
    }
}

pub(super) enum UsageEvent {
    ReInvite(IncomingRequest),
    Bye(IncomingRequest),
}
