// [[vk::push_constant]]
// struct PushConstants {
//     float4x4 view_projection;
//     float4x4 transform;
// } push_constants;

struct PsInput {
    float4 position : SV_POSITION;
    float2 texcoord : TEXCOORD;
    float3 normal : NORMAL;
};

PsInput vs_main(
    float3 position : POSITION,
    float3 normal : NORMAL,
    float2 texcoord : TEXCOORD
) {
    PsInput result;
    result.position = float4(position, 1.0);
    result.normal = normal;
    result.texcoord = texcoord;
    // result.position = mul(push_constants.view_projection, mul(push_constants.transform, float4(position, 1.0)));
    // result.texcoord = texcoord;
    // result.normal = mul((float4x3)push_constants.transform, normal);
    return result;
}

float4 fs_main(PsInput input) : SV_TARGET {
    float3 sun_dir = normalize(float3(0.7, 0.8, 0.3));
    float3 sun_color = float3(1.0, 1.0, 1.0);

    float3 albedo = float3(1.0, 1.0, 1.0);
    float env = 0.4;

    float n_dot_l = dot(input.normal, normalize(sun_dir));

    float3 shaded = saturate(env + albedo * sun_color * (saturate(n_dot_l) - 0.5 * env));

    return float4(shaded, 1.0);
}
