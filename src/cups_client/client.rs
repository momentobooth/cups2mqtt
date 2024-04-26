use std::{collections::HashMap, io::Cursor};
use chrono::DateTime;
use ipp::prelude::*;
use url::Url;
use anyhow::{Context, Result};

use crate::config::models::Cups;

use super::models::{*};

pub fn build_cups_url(cups_settings: &Cups, queue_id: Option<String>) -> String {
    let mut cups_url = Url::parse(&cups_settings.uri).unwrap();
    if !cups_settings.username.is_empty() && !cups_settings.password.is_empty() {
        cups_url.set_username(&cups_settings.username).unwrap();
        cups_url.set_password(Some(&cups_settings.password)).unwrap();
    }

    match queue_id {
        Some(queue_id) => cups_url.join("printers/").unwrap().join(&queue_id).unwrap(),
        None => cups_url,
    }.to_string()
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

/// Send an IPP request to do `op` to job `job_id` to the given `uri` and get the response.
///
/// # Arguments
///
/// * `uri`: Printer or server URI
/// * `op`: Operation
/// * `job_id`: Job id
///
/// returns: Result<IppRequestResponse, IppError>
///
/// # Examples
///
/// ```
/// send_ipp_job_request(uri, Operation::RestartJob, job_id).header().status_code().is_success()
/// ```
fn send_ipp_job_request(uri: String, ignore_tls_errors: bool, op: Operation, job_id: i32) -> Result<IppRequestResponse> {
    let uri_p: Uri = uri.parse()?;
    let mut req = IppRequestResponse::new(
        IppVersion::v1_1(),
        op,
        Some(uri_p.clone())
    );
    req.attributes_mut().add(
        DelimiterTag::OperationAttributes,
        IppAttribute::new(IppAttribute::JOB_ID, IppValue::Integer(job_id)),
    );

    let client = IppClient::builder(uri_p).ignore_tls_errors(ignore_tls_errors).build();
    let resp = client.send(req);
    Ok(resp?)
}

pub fn resume_printer(uri: String, ignore_tls_errors: bool) -> Result<bool> {
    Ok(send_ipp_request(uri, ignore_tls_errors, Operation::ResumePrinter)?.header().status_code().is_success())
}

pub fn purge_jobs(uri: String, ignore_tls_errors: bool) -> Result<bool> {
    Ok(send_ipp_request(uri, ignore_tls_errors, Operation::PurgeJobs)?.header().status_code().is_success())
}

pub fn print_job(uri: String, ignore_tls_errors: bool, job_name: String, pdf_data: Vec<u8>) -> bool {
    let uri_p: Uri = uri.parse().unwrap();
    let pdf_data_cursor = Cursor::new(pdf_data);
    let pdf_data_payload = IppPayload::new(pdf_data_cursor);
    let print_job = IppOperationBuilder::print_job(uri_p.clone(), pdf_data_payload).job_title(job_name);

    let client = IppClient::builder(uri_p).ignore_tls_errors(ignore_tls_errors).build();
    let resp = client.send(print_job.build());
    resp.unwrap().header().status_code().is_success()
}

pub fn restart_job(uri: String, ignore_tls_errors: bool, job_id: i32) -> Result<bool> {
    Ok(send_ipp_job_request(uri, ignore_tls_errors, Operation::RestartJob, job_id)?.header().status_code().is_success())
}

pub fn release_job(uri: String, ignore_tls_errors: bool, job_id: i32) -> Result<bool> {
    Ok(send_ipp_job_request(uri, ignore_tls_errors, Operation::ReleaseJob, job_id)?.header().status_code().is_success())
}

pub fn cancel_job(uri: String, ignore_tls_errors: bool, job_id: i32) -> Result<bool> {
    Ok(send_ipp_job_request(uri, ignore_tls_errors, Operation::CancelJob, job_id)?.header().status_code().is_success())
}

pub fn get_printers(uri: String, ignore_tls_errors: bool) -> Result<Vec<IppPrinterState>> {
    let resp = send_ipp_request(uri.clone(), ignore_tls_errors, Operation::CupsGetPrinters);
    let mut vec: Vec<IppPrinterState> = Vec::new();

    for printer in resp?.attributes().groups_of(DelimiterTag::PrinterAttributes) {
        let group = printer.attributes().clone();
        let state = group["printer-state"]
            .value()
            .as_enum()
            .and_then(|v| PrinterState::from_i32(*v))
            .unwrap();
        let job_count = group["queued-job-count"].value().as_integer().unwrap().clone();
        let state_message = group["printer-state-message"].value().to_string().clone();
        let queue_name = group["printer-name"].value().to_string().clone();
        let description = group["printer-info"].value().to_string().clone();
        let state_reason = group["printer-state-reasons"].value().to_string().clone();
        vec.push(IppPrinterState { queue_name, description, state, job_count, state_message, state_reason });
    }

    Ok(vec)
}

pub fn get_printer_state(uri: String, ignore_tls_errors: bool) -> Result<IppPrinterState> {
    let resp = send_ipp_request(uri.clone(), ignore_tls_errors, Operation::GetPrinterAttributes)?;

    let group = resp.attributes().groups_of(DelimiterTag::PrinterAttributes).next().context("Invalid group returned")?;
    let attributes = group.attributes().clone();

    let state = group.attributes()["printer-state"]
        .value()
        .as_enum()
        .and_then(|v| PrinterState::from_i32(*v))
        .unwrap();
    let job_count = attributes["queued-job-count"].value().as_integer().context("Could not parse queued-job-count to i32")?.clone();
    let state_message = attributes["printer-state-message"].value().to_string().clone();
    let queue_name = attributes["printer-name"].value().to_string().clone();
    let description = attributes["printer-info"].value().to_string().clone();
    let state_reason = attributes["printer-state-reasons"].value().to_string().clone();
    //print_attributes(attributes);
    Ok(IppPrinterState { queue_name, description, state, job_count, state_message, state_reason })
}

pub fn get_jobs_states(uri: String, ignore_tls_errors: bool) -> Result<Vec<PrintJobState>> {
    let resp = send_ipp_request(uri.clone(), ignore_tls_errors, Operation::GetJobs);
    let mut vec: Vec<PrintJobState> = Vec::new();

    for job in resp?.attributes().groups_of(DelimiterTag::JobAttributes) {
        let job_id = job.attributes()["job-id"].value().as_integer().context("Could not convert job-id to i32")?.clone();
        vec.push(get_job_state(uri.clone(), ignore_tls_errors, job_id)?);
    }

    // print_attributes(attributes);
    Ok(vec)
}

fn get_job_state(uri: String, ignore_tls_errors: bool, job_id: i32) -> Result<PrintJobState> {
    let resp = send_ipp_job_request(uri.clone(), ignore_tls_errors, Operation::GetJobAttributes, job_id)?;

    let group = resp.attributes().groups_of(DelimiterTag::JobAttributes).next().context("Invalid group returned")?;
    let attributes = group.attributes().clone();

    // print_attributes(attributes.clone());

    let state = group.attributes()["job-state"]
        .value()
        .as_enum()
        .and_then(|v| JobState::from_i32(*v))
        .context("Could not convert i32 value to JobState")?;

    let creation_time = DateTime::from_timestamp_millis((attributes["time-at-creation"].value().as_integer().context("Could not convert time-at-creation to i32")?.clone() as i64) * 1000).unwrap();

    // Not every job seems to have a name
    let job_name = if attributes.get_key_value("job-name").is_some() { attributes["job-name"].value().to_string().clone() } else { "".to_string() };

    Ok(PrintJobState {
        name: job_name,
        id: attributes["job-id"].value().as_integer().unwrap().clone(),
        state: state,
        reason: attributes["job-state-reasons"].value().to_string().clone(),
        created: creation_time,
    })
}

fn print_attributes(attributes: HashMap<String, IppAttribute>) {
    for attribute in attributes {
        println!("Attribute {}: {:?}", attribute.0, attribute.1.value());
    }
}
