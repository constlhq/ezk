use super::{Dialog, DialogLayer};
use crate::dialog::layer::DialogEntry;
use crate::util::{random_sequence_number, random_string};
use bytes::Bytes;
use sip_core::transaction::TsxResponse;
use sip_core::transport::TargetTransportInfo;
use sip_core::{Endpoint, Request};
use sip_types::header::HeaderError;
use sip_types::header::typed::{CSeq, CallID, Contact, FromTo, MaxForwards};
use sip_types::msg::RequestLine;
use sip_types::uri::{NameAddr, SipUri};
use sip_types::{Headers, Method, Name};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct ClientDialogBuilder {
    pub endpoint: Endpoint,
    pub local_cseq: u32,
    pub local_fromto: FromTo,
    pub peer_fromto: FromTo,
    pub local_contact: Contact,
    pub call_id: CallID,
    pub target: SipUri,
    pub secure: bool,
    pub target_tp_info: TargetTransportInfo,
}

impl ClientDialogBuilder {
    pub fn new(
        endpoint: Endpoint,
        local_addr: NameAddr,
        local_contact: Contact,
        target: SipUri,
    ) -> Self {
        Self {
            endpoint,
            local_cseq: random_sequence_number(),
            local_fromto: FromTo::new(local_addr, Some(random_string())),
            peer_fromto: FromTo::new(NameAddr::uri(target.clone()), None),
            local_contact,
            call_id: CallID(random_string()),
            secure: target.sips,
            target,
            target_tp_info: TargetTransportInfo::default(),
        }
    }

    pub fn create_request(&mut self, method: Method) -> Request {
        let mut headers = Headers::new();

        headers.insert_named(&MaxForwards(70));
        headers.insert_type(Name::FROM, &self.local_fromto);
        headers.insert_type(Name::TO, &self.peer_fromto);
        headers.insert_named(&self.call_id);
        headers.insert_named(&CSeq {
            cseq: self.local_cseq,
            method: method.clone(),
        });
        headers.insert_named(&self.local_contact);

        Request {
            line: RequestLine {
                method,
                uri: self.target.clone(),
            },
            headers,
            body: Bytes::new(),
        }
    }

    pub fn create_dialog_from_response(
        &mut self,
        response: &TsxResponse,
    ) -> Result<Dialog, HeaderError> {
        assert!(response.base_headers.to.tag.is_some());

        let dialog = Dialog {
            endpoint: self.endpoint.clone(),
            local_cseq: self.local_cseq.into(),
            local_fromto: self.local_fromto.clone(),
            peer_fromto: response.base_headers.to.clone(),
            local_contact: self.local_contact.clone(),
            peer_contact: response.headers.get_named()?,
            call_id: self.call_id.clone(),
            route_set: response.headers.get(Name::RECORD_ROUTE).unwrap_or_default(),
            secure: self.secure,
            target_tp_info: Mutex::new(self.target_tp_info.clone()),
        };

        let entry = DialogEntry::new(None);
        self.endpoint
            .layer::<DialogLayer>()
            .dialogs
            .lock()
            .insert(dialog.key(), entry);

        Ok(dialog)
    }
}
