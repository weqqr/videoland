struct PsInput {
    float4 position : SV_POSITION;
    float2 texcoord : TEXCOORD;
    float4 color : COLOR;
};

PsInput vs_main(
    float2 position : SV_POSITION,
    float2 texcoord : TEXCOORD,
    uint color : COLOR,
) {
    PsInput result;
    result.position = float4(position, 0.0, 1.0);
    result.texcoord = texcoord;
    result.color = float4(1.0, 1.0, 1.0, 1.0);
    return result;
}

float4 fs_main(PsInput input) : SV_TARGET {
    return input.color;
}
