struct Uniforms {
    float2 viewport_size;
} uniforms : register(b0);

struct PsInput {
    float4 position : SV_POSITION;
    float2 texcoord : TEXCOORD;
    float4 color : COLOR;
};

float4 decode_color(uint rgba) {
    uint4 color = uint4(rgba >> 0, rgba >> 8, rgba >> 16, rgba >> 24);
    return float4(color & 0xFF) / 255.0;
}

PsInput vs_main(
    float2 position : POSITION,
    float2 texcoord : TEXCOORD,
    uint color : COLOR
) {
    float2 normalized_position = (2 * float2(1, -1) * position - float2(0.5, 0.5)) / uniforms.viewport_size + float2(-1, 1);

    PsInput result;
    result.position = float4(normalized_position, 0.0, 1.0);
    result.texcoord = texcoord;
    result.color = decode_color(color);
    return result;
}

float4 fs_main(PsInput input) : SV_TARGET {
    return input.color;
}
