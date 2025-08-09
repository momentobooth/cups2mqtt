use std::{collections::HashMap, io::Cursor};

use ipp::prelude::*;
use snafu::{whatever, OptionExt, ResultExt, Snafu};
use url::Url;

use crate::config::models::Cups;

use super::models::{*};

// //////////// //
// Print queues //
// //////////// //

pub async fn get_raw_print_queues(uri: String, ignore_tls_errors: bool) -> Result<IppRequestResponse, CupsError> {
    send_ipp_request(uri.clone(), ignore_tls_errors, Operation::CupsGetPrinters).await
}

pub async fn get_print_queues(uri: String, ignore_tls_errors: bool) -> Result<Vec<IppPrintQueueState>, CupsError> {
    let resp = get_raw_print_queues(uri, ignore_tls_errors).await;
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

        let mut markers = Vec::<IppPrinterMarker>::new();

        // Here use `.get` instead of the index to avoid a crash, as
        // these values are not always available for various reasons.
        let marker_types = get_ipp_strings(&group, "marker-types");
        let marker_colors = get_ipp_strings(&group, "marker-colors");
        let marker_names = get_ipp_strings(&group, "marker-names");
        let marker_levels = get_ipp_ints(&group, "marker-levels");

        if marker_types.is_ok() && marker_colors.is_ok() && marker_names.is_ok() && marker_levels.is_ok() {
            let marker_types = marker_types.unwrap();
            let marker_colors = marker_colors.unwrap();
            let marker_names = marker_names.unwrap();
            let marker_levels = marker_levels.unwrap();

            for i in 0..marker_types.len() {
                let marker_level = marker_levels[i];
                let marker_color = marker_colors[i].clone();

                markers.push(IppPrinterMarker {
                    marker_type: marker_types[i].clone(),
                    color: match &marker_color {
                        _ if marker_color == "none" => None,
                        _ => Some(marker_color),
                    },
                    name: marker_names[i].to_string(),
                    level: match marker_level {
                        -1 => None,
                        _ => Some(marker_level as u32),
                    },
                });
            }
        }

        vec.push(IppPrintQueueState { queue_name, description, printer_make, state, job_count, state_message, state_reason, cups_version, markers });
    }

    Ok(vec)
}

// ///////////////////// //
// Printing and commands //
// ///////////////////// //

pub async fn report_supply_levels(uri: String, ignore_tls_errors: bool) -> Result<(), CupsError> {
    let command = "#CUPS-COMMAND\nReportLevels";
    let command_bytes: Vec<u8> = command.as_bytes().to_vec();
    print_job(uri, ignore_tls_errors, "CUPS2MQTT update supply levels".to_owned(), command_bytes).await?;
    Ok(())
}

pub async fn print_job(uri: String, ignore_tls_errors: bool, job_name: String, job_data: Vec<u8>) -> Result<(), CupsError> {
    let uri_p: Uri = uri.parse::<Uri>().with_whatever_context(|_| format!("Could not parse URI {uri}"))?.clone();
    let pdf_data_cursor = Cursor::new(job_data);
    let pdf_data_payload = IppPayload::new(pdf_data_cursor);
    let print_job = IppOperationBuilder::print_job(uri_p.clone(), pdf_data_payload).job_title(job_name);

    let client = AsyncIppClient::builder(uri_p).ignore_tls_errors(ignore_tls_errors).build();
    let resp = client.send(print_job.build()).await.with_whatever_context(|_| format!("IPP request failed"))?;
    if !resp.header().status_code().is_success() {
        whatever!("IPP request failed with status code [{}]", resp.header().status_code())
    }
    Ok(())
}

// /////// //
// Helpers //
// /////// //

fn get_ipp_strings(ipp_group: &HashMap<String, IppAttribute>, value_name: &str) -> Result<Vec<String>, CupsError> {
    Ok(
        ipp_group.get(value_name)
            .with_whatever_context(|| format!("Value {value_name} not found in group"))?
            .value().clone().as_array()
            .with_whatever_context(|| format!("Could not read IPP value as array"))?
            .iter().map(|f| f.to_string()).collect()
    )
}

fn get_ipp_ints(ipp_group: &HashMap<String, IppAttribute>, value_name: &str) -> Result<Vec<i32>, CupsError> {
    Ok(
        ipp_group.get(value_name)
            .with_whatever_context(|| format!("Value {value_name} not found in group"))?
            .value().clone().as_array()
            .with_whatever_context(|| format!("Could not read IPP value as array"))?
            .iter().map(|f| f.as_integer().unwrap().to_owned()).collect()
    )
}

pub fn build_cups_url(cups_settings: &Cups, queue_id: Option<&String>) -> Result<String, CupsError> {
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
