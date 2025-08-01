use std::io::Cursor;

use ipp::prelude::*;
use snafu::{OptionExt, ResultExt, Snafu};
use url::Url;

use crate::config::models::Cups;

use super::models::{*};

// //////////// //
// Print queues //
// //////////// //

pub async fn get_print_queues(uri: String, ignore_tls_errors: bool) -> Result<Vec<IppPrintQueueState>, CupsError> {
    let resp = send_ipp_request(uri.clone(), ignore_tls_errors, Operation::CupsGetPrinters).await;
    let mut vec: Vec<IppPrintQueueState> = Vec::new();

    for printer in resp?.attributes().groups_of(DelimiterTag::PrinterAttributes) {
        let group = printer.attributes().clone();
        let state = group["printer-state"]
            .value()
            .as_enum()
            .and_then(|v| PrinterState::from_i32(*v)).with_whatever_context(|| "Failed to parse printer state")?;
        let job_count = group["queued-job-count"].value().as_integer().with_whatever_context(|| "Failed to parse job count")?.clone();
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

// ///////////////////// //
// Printing and commands //
// ///////////////////// //

pub async fn report_supply_levels(uri: String, ignore_tls_errors: bool) -> Result<bool, CupsError> {
    let command = "#CUPS-COMMAND\nReportLevels";
    let command_bytes: Vec<u8> = command.as_bytes().to_vec();
    Ok(print_job(uri, ignore_tls_errors, "CUPS2MQTT update supply levels".to_owned(), command_bytes).await?)
}

pub async fn print_job(uri: String, ignore_tls_errors: bool, job_name: String, job_data: Vec<u8>) -> Result<bool, CupsError> {
    let uri_p: Uri = uri.parse::<Uri>().with_whatever_context(|_| format!("Could not parse URI {uri}"))?.clone();
    let pdf_data_cursor = Cursor::new(job_data);
    let pdf_data_payload = IppPayload::new(pdf_data_cursor);
    let print_job = IppOperationBuilder::print_job(uri_p.clone(), pdf_data_payload).job_title(job_name);

    let client = AsyncIppClient::builder(uri_p).ignore_tls_errors(ignore_tls_errors).build();
    let resp = client.send(print_job.build()).await.with_whatever_context(|_| format!("Could not send print job"))?;
    Ok(resp.header().status_code().is_success())
}

// /////// //
// Helpers //
// /////// //

pub fn build_cups_url(cups_settings: &Cups, queue_id: Option<String>) -> Result<String, CupsError> {
    let mut cups_url = Url::parse(&cups_settings.uri).with_whatever_context(|_| "Could not parse CUPS URI")?;
    if !cups_settings.username.is_empty() && !cups_settings.password.is_empty() {
        cups_url.set_username(&cups_settings.username).unwrap();
        cups_url.set_password(Some(&cups_settings.password)).unwrap();
    }

    Ok(match queue_id {
        Some(queue_id) => cups_url.join("printers/").with_whatever_context(|_| "Could join ./printers/")?.join(&queue_id).with_whatever_context(|_| format!("Could not join queue ID {queue_id}"))?,
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
async fn send_ipp_request(uri: String, ignore_tls_errors: bool, op: Operation) -> Result<IppRequestResponse, CupsError> {
    let uri_p: Uri = uri.parse().with_whatever_context(|_| format!("Could not parse URI {uri}"))?;
    let req = IppRequestResponse::new(
        IppVersion::v2_2(),
        op,
        Some(uri_p.clone())
    );

    // If we ever want to specify which attributes we want to receive.
    // req.attributes_mut().groups_mut().first_mut().unwrap().attributes_mut().insert("requested-attributes".to_owned(), IppAttribute::new(IppAttribute::REQUESTED_ATTRIBUTES, IppValue::Array(vec![
    //     IppValue::Keyword("printer-name".to_owned())
    // ])));

    let client = AsyncIppClient::builder(uri_p).ignore_tls_errors(ignore_tls_errors).build();
    Ok(client.send(req).await.with_whatever_context(|_| format!("Could not send IPP request"))?)
}

// ////// //
// Errors //
// ////// //

#[derive(Debug, Snafu)]
pub enum CupsError {
    #[snafu(whatever, display("{message}"))]
    Whatever {
        message: String,
        #[snafu(source(from(Box<dyn std::error::Error + Send + Sync>, Some)))]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}
