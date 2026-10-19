use crate::error::Result;
use log::info;
use serde_json::Value;
use std::collections::BTreeMap;
use yaak_models::models::{Environment, HttpRequestHeader, HttpUrlParameter, WebsocketRequest};
use yaak_models::render::{
    join_request_prefix, make_vars_hashmap, request_prefix_from_environment_chain,
};
use yaak_templates::{RenderOptions, TemplateCallback, parse_and_render, render_json_value_raw};

pub async fn render_websocket_request<T: TemplateCallback>(
    r: &WebsocketRequest,
    environment_chain: Vec<Environment>,
    cb: &T,
    opt: &RenderOptions,
) -> Result<WebsocketRequest> {
    let request_prefix = request_prefix_from_environment_chain(&environment_chain).to_string();
    let vars = &make_vars_hashmap(environment_chain);

    let mut url_parameters = Vec::new();
    for p in r.url_parameters.clone() {
        if !p.enabled {
            continue;
        }
        url_parameters.push(HttpUrlParameter {
            enabled: p.enabled,
            name: parse_and_render(&p.name, vars, cb, opt).await?,
            value: parse_and_render(&p.value, vars, cb, opt).await?,
            id: p.id,
        })
    }

    let mut headers = Vec::new();
    for p in r.headers.clone() {
        if !p.enabled {
            continue;
        }
        headers.push(HttpRequestHeader {
            enabled: p.enabled,
            name: parse_and_render(&p.name, vars, cb, opt).await?,
            value: parse_and_render(&p.value, vars, cb, opt).await?,
            id: p.id,
        })
    }

    let authentication = {
        let mut disabled = false;
        let mut auth = BTreeMap::new();
        match r.authentication.get("disabled") {
            Some(Value::Bool(true)) => {
                disabled = true;
            }
            Some(Value::String(tmpl)) => {
                disabled = parse_and_render(tmpl.as_str(), vars, cb, &opt)
                    .await
                    .unwrap_or_default()
                    .is_empty();
                info!(
                    "Rendering authentication.disabled as a template: {disabled} from \"{tmpl}\""
                );
            }
            _ => {}
        }
        if disabled {
            auth.insert("disabled".to_string(), Value::Bool(true));
        } else {
            for (k, v) in r.authentication.clone() {
                if k == "disabled" {
                    auth.insert(k, Value::Bool(false));
                } else {
                    auth.insert(k, render_json_value_raw(v, vars, cb, &opt).await?);
                }
            }
        }
        auth
    };

    let url = parse_and_render(r.url.as_str(), vars, cb, opt).await?;
    let prefix = parse_and_render(&request_prefix, vars, cb, opt).await?;
    let url = join_request_prefix(&prefix, &url);

    let mut stomp_subscriptions = Vec::new();
    for destination in r.stomp_subscriptions.clone() {
        stomp_subscriptions.push(parse_and_render(&destination, vars, cb, opt).await?)
    }
    let stomp_destination = parse_and_render(&r.stomp_destination, vars, cb, opt).await?;

    let message = parse_and_render(&r.message.clone(), vars, cb, opt).await?;

    Ok(WebsocketRequest {
        url,
        url_parameters,
        headers,
        authentication,
        message,
        stomp_destination,
        stomp_subscriptions,
        ..r.to_owned()
    })
}
