use serde_json::{Value, json};

use super::*;

fn ask(message: &Value) -> Value {
    answer(&message.to_string()).expect("a reply")
}

#[test]
fn it_introduces_itself_and_lists_its_tools() {
    let hello = ask(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}));
    assert_eq!(hello["result"]["protocolVersion"], PROTOCOL);
    assert!(answer(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
    let listed = ask(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    let names: Vec<_> = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "project_check",
            "play_run",
            "project_overview",
            "scene_outline",
            "find_entities",
            "show_entity",
            "asset_uses",
            "style_show",
            "style_match",
            "style_unused",
            "check_script",
            "decay_api",
            "capabilities",
            "capture"
        ]
    );
}

#[test]
fn a_tool_answers_with_its_command_s_json() {
    let project = concat!(env!("CARGO_MANIFEST_DIR"), "/../../games/flappy");
    let checked = ask(&json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
        "params": {"name": "project_check", "arguments": {"project": project}}}));
    assert_eq!(checked["result"]["isError"], false);
    assert_eq!(checked["result"]["structuredContent"]["schema_version"], 1);
    assert_eq!(checked["result"]["structuredContent"]["ok"], true);
    let played = ask(&json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call",
        "params": {"name": "play_run", "arguments": {"project": project, "steps": ["wait:0.2"]}}}));
    assert_eq!(played["result"]["structuredContent"]["frames"], 12);
}

#[test]
fn it_hands_out_the_schemas_the_engine_writes() {
    let schema = ask(&json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call",
        "params": {"name": "capabilities", "arguments": {"name": "scene"}}}));
    assert_eq!(schema["result"]["isError"], false);
    assert_eq!(
        schema["result"]["structuredContent"]["title"],
        "Sindri scene"
    );
    let nothing = ask(&json!({"jsonrpc": "2.0", "id": 8, "method": "tools/call",
        "params": {"name": "capabilities", "arguments": {"name": "gossip"}}}));
    assert_eq!(nothing["result"]["isError"], true);
}

#[test]
fn the_questions_answer_with_their_command_s_json() {
    let project = concat!(env!("CARGO_MANIFEST_DIR"), "/../../games/flappy");
    let ask_tool = |name: &str, arguments: Value| {
        let mut arguments = arguments;
        arguments["project"] = json!(project);
        ask(&json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call",
            "params": {"name": name, "arguments": arguments}}))["result"]
            .clone()
    };
    let overview = ask_tool("project_overview", json!({}));
    assert_eq!(overview["structuredContent"]["main_scene"], "flappy.scene");
    let outline = ask_tool("scene_outline", json!({}));
    assert!(outline["structuredContent"]["entities"].as_u64().unwrap() > 5);
    let birds = ask_tool("find_entities", json!({"script": "Bird"}));
    let bird = birds["structuredContent"]["found"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let shown = ask_tool("show_entity", json!({"id": bird}));
    assert_eq!(shown["isError"], false, "{shown}");
    let missing = ask_tool("show_entity", json!({}));
    assert_eq!(missing["isError"], true);
    let uses = ask_tool("asset_uses", json!({"asset": "scripts/bird.decay"}));
    assert!(
        !uses["structuredContent"]["uses"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn the_language_tools_answer_from_the_engine() {
    let call = |name: &str, arguments: Value| {
        ask(&json!({"jsonrpc": "2.0", "id": 10, "method": "tools/call",
            "params": {"name": name, "arguments": arguments}}))["result"]
            .clone()
    };
    let scorchball = concat!(env!("CARGO_MANIFEST_DIR"), "/../../games/scorchball");
    let reached = call(
        "style_match",
        json!({"project": scorchball, "selector": ".title-button"}),
    );
    assert_eq!(
        reached["structuredContent"]["found"]
            .as_array()
            .unwrap()
            .len(),
        2,
        "{reached}"
    );
    let styled = call(
        "style_show",
        json!({"project": scorchball, "id": "play-bot"}),
    );
    assert_eq!(
        styled["structuredContent"]["computed"]["background"], "#f26a16",
        "{styled}"
    );

    let flappy = concat!(env!("CARGO_MANIFEST_DIR"), "/../../games/flappy");
    let draft = "script Probe {\n    fn update(dt: number) {\n        World.despawn(this.entiy);\n    }\n}\n";
    let checked = call("check_script", json!({"project": flappy, "source": draft}));
    assert_eq!(checked["structuredContent"]["ok"], false);
    assert_eq!(checked["structuredContent"]["diagnostics"][0]["line"], 3);
    let fixed = call(
        "check_script",
        json!({"project": flappy, "source": draft.replace("entiy", "entity")}),
    );
    assert_eq!(fixed["structuredContent"]["ok"], true, "{fixed}");

    let api = call("decay_api", json!({"name": "Physics.mask"}));
    assert!(
        api["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Physics.mask(")
    );
    let near = call("decay_api", json!({"name": "this.entiy"}));
    assert_eq!(near["isError"], true);
    assert_eq!(near["structuredContent"]["nearest"][0], "this.entity");
}

#[test]
fn pictures_travel_as_base64() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    assert_eq!(base64(&[0xFF, 0xEE]), "/+4=");
}

#[test]
fn what_it_cannot_do_it_says() {
    let unknown = ask(&json!({"jsonrpc": "2.0", "id": 5, "method": "resources/write"}));
    assert_eq!(unknown["error"]["code"], -32601);
    let nowhere = ask(&json!({"jsonrpc": "2.0", "id": 6, "method": "tools/call",
        "params": {"name": "project_check", "arguments": {"project": "/"}}}));
    assert_eq!(nowhere["result"]["isError"], true);
    assert_eq!(answer("not json").unwrap()["error"]["code"], -32700);
}
