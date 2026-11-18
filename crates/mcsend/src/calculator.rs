//! The three operations a send needs: device information, the installed apps, and the transfer.

use crate::error::Error;
use crate::message::{
    ATTR_ARCHIVED, ATTR_VAR_SIZE, ATTR_VAR_TYPE, PARAM_FREE_FLASH, PARAM_PRODUCT_NAME,
    PARAM_PRODUCT_NUMBER, TYPE_APP, be_number, dir_request, param_request, parse_params,
    parse_var_header, request_to_send_app,
};
use crate::packet::vtype;
use crate::session::{Session, into_expected};
use crate::tifile::App;
use crate::transport::Transport;

/// Product number of the 84 Plus family in the parameter data.
pub const PRODUCT_84_PLUS: u64 = 0x0A;

#[derive(Debug)]
pub struct Info {
    pub product: u64,
    pub name: String,
    pub free_flash: Option<u64>,
}

/// Asks for the model and the free flash. The session must have begun.
pub fn info<T: Transport>(session: &mut Session<T>) -> Result<Info, Error> {
    let ids = [PARAM_PRODUCT_NUMBER, PARAM_PRODUCT_NAME, PARAM_FREE_FLASH];
    session.send_virtual(vtype::PARAM_REQUEST, &param_request(&ids), &mut |_| {})?;
    let params = parse_params(&session.expect(vtype::PARAM_DATA)?)?;
    let value = |id: u16| {
        params
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| p.value.as_deref())
    };
    let product = value(PARAM_PRODUCT_NUMBER)
        .and_then(be_number)
        .ok_or_else(|| Error::Protocol("no product number".into()))?;
    let name = value(PARAM_PRODUCT_NAME)
        .map(|v| {
            String::from_utf8_lossy(v)
                .trim_end_matches('\0')
                .to_string()
        })
        .unwrap_or_default();
    Ok(Info {
        product,
        name,
        free_flash: value(PARAM_FREE_FLASH).and_then(be_number),
    })
}

/// Names of the apps installed on the calculator. The session must have begun.
pub fn installed_apps<T: Transport>(session: &mut Session<T>) -> Result<Vec<String>, Error> {
    session.send_virtual(
        vtype::DIR_REQUEST,
        &dir_request(&[ATTR_VAR_SIZE, ATTR_VAR_TYPE, ATTR_ARCHIVED]),
        &mut |_| {},
    )?;
    let mut apps = Vec::new();
    loop {
        let packet = session.recv_virtual()?;
        if packet.kind == vtype::EOT {
            return Ok(apps);
        }
        let header = parse_var_header(&into_expected(packet, vtype::VAR_HEADER)?)?;
        let is_app = header
            .attrs
            .iter()
            .find(|a| a.id == ATTR_VAR_TYPE)
            .and_then(|a| a.value.as_deref())
            .is_some_and(|v| v.last() == Some(&TYPE_APP));
        if is_app {
            apps.push(header.name);
        }
    }
}

/// Sends the app and stops after the end of transmission, as the calculator says nothing more.
/// A failure from the first content packet on is wrapped in `Error::Send`.
/// The session must have begun. `progress` gets the bytes of the app image acknowledged so far.
pub fn send_app<T: Transport>(
    session: &mut Session<T>,
    app: &App,
    progress: &mut dyn FnMut(usize),
) -> Result<(), Error> {
    let image = app.image();
    let request = request_to_send_app(&app.name, image.len() as u32);
    session.send_virtual(vtype::REQUEST_TO_SEND, &request, &mut |_| {})?;
    session.expect(vtype::DATA_ACK).map_err(|e| match e {
        Error::Calculator(code) => Error::Refused {
            code,
            name: app.name.clone(),
        },
        other => other,
    })?;
    let mut content = || {
        session.send_virtual(vtype::VAR_CONTENT, &image, progress)?;
        session.expect(vtype::DATA_ACK)?;
        session.send_virtual(vtype::EOT, &[], &mut |_| {})
    };
    content().map_err(|e| Error::Send(Box::new(e)))
}
