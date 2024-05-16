use std::io::{Cursor, Read};

use ipp::prelude::*;
use url::Url;
use anyhow::{Context, Result};

use crate::config::models::Cups;

use super::models::{*};

// //////////// //
// Print queues //
// //////////// //

pub fn get_print_queues(uri: String, ignore_tls_errors: bool) -> Result<Vec<IppPrintQueueState>> {
    let resp = send_ipp_request(uri.clone(), ignore_tls_errors, Operation::CupsGetPrinters);
    let mut vec: Vec<IppPrintQueueState> = Vec::new();

    for printer in resp?.attributes().groups_of(DelimiterTag::PrinterAttributes) {
        let group = printer.attributes().clone();
        let state = group["printer-state"]
            .value()
            .as_enum()
            .and_then(|v| PrinterState::from_i32(*v)).context("Failed to parse printer state")?;
        let job_count = group["queued-job-count"].value().as_integer().context("Failed to parse job count")?.clone();
        let state_message = group["printer-state-message"].value().to_string().clone();
        let queue_name = group["printer-name"].value().to_string().clone();
        let description = group["printer-info"].value().to_string().clone();
        let printer_make = group["printer-make-and-model"].value().to_string().clone();
        let state_reason = group["printer-state-reasons"].value().to_string().clone();
        let cups_version = group["cups-version"].value().to_string().clone();
        vec.push(IppPrintQueueState { queue_name, description, printer_make, state, job_count, state_message, state_reason, cups_version });
    }

    Ok(vec)
}

pub fn print_job(uri: String, ignore_tls_errors: bool, job_name: String, pdf_data: Vec<u8>) -> bool {
    let uri_p: Uri = uri.parse().unwrap();
    let pdf_data_cursor = Cursor::new(pdf_data);
    let pdf_data_payload = IppPayload::new(pdf_data_cursor);
    let print_job = IppOperationBuilder::print_job(uri_p.clone(), pdf_data_payload).job_title(job_name);

    let client = IppClient::builder(uri_p).ignore_tls_errors(ignore_tls_errors).build();
    let resp = client.send(print_job.build());
    println!("{:?}", resp.as_ref().unwrap().attributes());
    resp.unwrap().header().status_code().is_success()
}

// /////// //
// Helpers //
// /////// //

pub fn build_cups_url(cups_settings: &Cups, queue_id: Option<String>) -> Result<String> {
    let mut cups_url = Url::parse(&cups_settings.uri)?;
    if !cups_settings.username.is_empty() && !cups_settings.password.is_empty() {
        cups_url.set_username(&cups_settings.username).unwrap(); // FIXME: use .? instead of .unwrap()
        cups_url.set_password(Some(&cups_settings.password)).unwrap(); // FIXME: use .? instead of .unwrap()
    }

    Ok(match queue_id {
        Some(queue_id) => cups_url.join("printers/")?.join(&queue_id)?,
        None => cups_url,
    }.to_string())
}

/// Send an IPP request to do `op` to the given `uri` and get the response.
///
/// # Arguments
///
/// * `uri`: Printer or server URI
/// * `op`: Operation
///
/// returns: Result<IppRequestResponse, IppError>
///
/// # Examples
///
/// ```
/// send_ipp_request(uri, Operation::ResumePrinter).header().status_code().is_success()
/// ```
fn send_ipp_request(uri: String, ignore_tls_errors: bool, op: Operation) -> Result<IppRequestResponse> {
    let uri_p: Uri = uri.parse()?;
    let req = IppRequestResponse::new(
        IppVersion::v1_1(),
        op,
        Some(uri_p.clone())
    );
    let client = IppClient::builder(uri_p).ignore_tls_errors(ignore_tls_errors).build();
    let resp = client.send(req);
    Ok(resp?)
}
