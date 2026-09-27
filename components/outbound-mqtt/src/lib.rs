use anyhow::Context as _;
use helper::bindings::{
    fermyon::spin2_0_0::mqtt::{self, Qos},
    wasi::http0_2_0::types::{IncomingRequest, OutgoingResponse, ResponseOutparam},
};

struct Component;
helper::gen_http_trigger_bindings!(Component);

impl bindings::Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        helper::handle_result(handle(request), response_out)
    }
}

const MQTT_ADDRESS: &str = "MQTT_ADDRESS";
const MQTT_USERNAME: &str = "MQTT_USERNAME";
const MQTT_PASSWORD: &str = "MQTT_PASSWORD";

fn handle(request: IncomingRequest) -> anyhow::Result<OutgoingResponse> {
    let address = get_header(&request, MQTT_ADDRESS)?;
    let username = get_header(&request, MQTT_USERNAME)?;
    let password = get_header(&request, MQTT_PASSWORD)?;

    let connection = mqtt::Connection::open(&address, &username, &password, 30)?;

    connection.publish("telemetry-topic", b"Eureka!".as_ref(), Qos::AtLeastOnce)?;

    Ok(helper::ok_response())
}

fn get_header(request: &IncomingRequest, header_key: &str) -> anyhow::Result<String> {
    helper::get_header(request, header_key).with_context(|| format!("no {header_key} header"))
}
