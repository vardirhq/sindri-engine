use super::*;

#[test]
fn the_built_in_body_is_a_valid_sprite_shader() {
    SpriteShader::new("built-in", BUILT_IN).expect("the built-in look compiles");
}

#[test]
fn a_shader_using_the_whole_surface_compiles() {
    let source = "\
fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    let texel = sprite_texel_size();
    let right = sample_sprite(input.uv + vec2<f32>(texel.x, 0.0));
    let wave = sin(input.time + input.world_position.x) * input.params.x;
    return vec4<f32>(input.color.rgb + right.rgb * wave, input.color.a * input.local_uv.x);
}
";
    SpriteShader::new("shaders/all.wgsl", source).expect("the documented surface compiles");
}

#[test]
fn a_syntax_error_names_the_projects_own_line() {
    let source = "\
fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    let broken = ;
    return input.color;
}
";
    let error = SpriteShader::new("shaders/broken.wgsl", source).unwrap_err();
    let SpriteShaderError::Invalid { line, name, .. } = &error;
    assert_eq!(name, "shaders/broken.wgsl");
    assert_eq!(*line, Some(2), "{error}");
    assert!(
        error.to_string().starts_with("shaders/broken.wgsl:2: "),
        "{error}"
    );
}

#[test]
fn a_type_error_names_a_line_of_the_projects_own() {
    let source = "\
fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    return input.uv;
}
";
    let error = SpriteShader::new("shaders/typed.wgsl", source).unwrap_err();
    let SpriteShaderError::Invalid { line, .. } = &error;
    // Validation places the fault on the function it is in rather than on the
    // statement, so the function's own first line is as close as it gets.
    assert!(matches!(line, Some(1..=2)), "{error}");
}

#[test]
fn a_missing_entry_function_says_so() {
    let error = SpriteShader::new(
        "shaders/empty.wgsl",
        "fn not_it(input: SpriteShaderInput) -> vec4<f32> { return input.color; }\n",
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("does not define `fn sprite_shader"),
        "{error}"
    );
}

#[test]
fn a_commented_out_entry_function_does_not_count() {
    let error = SpriteShader::new(
        "shaders/commented.wgsl",
        "// fn sprite_shader(input: SpriteShaderInput) -> vec4<f32>\n",
    )
    .unwrap_err();
    assert!(error.to_string().contains("does not define"), "{error}");
}

#[test]
fn each_accepted_load_is_a_new_revision() {
    let first = SpriteShader::new("shaders/a.wgsl", BUILT_IN).unwrap();
    let second = SpriteShader::new("shaders/a.wgsl", BUILT_IN).unwrap();
    assert_ne!(first.revision(), second.revision());
    assert_ne!(first, second);
    assert_eq!(first, first.clone());
}

#[test]
fn a_source_without_a_final_newline_still_composes() {
    let source = "fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> { return input.color; }";
    SpriteShader::new("shaders/tight.wgsl", source).expect("no trailing newline is fine");
}
