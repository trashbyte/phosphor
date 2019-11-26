const mat3 RGB_TO_XYZ_D50 = (mat3(
0.4360747, 0.2225045, 0.0139322,
0.3850649, 0.7168786, 0.0971045,
0.1430804, 0.0606169, 0.7141733
));
const mat3 XYZ_TO_RGB_D50 = (mat3(
3.1338561, -0.9787684,  0.0719453,
-1.6168667,  1.9161415, -0.2289914,
-0.4906146,  0.0334540,  1.4052427
));

const mat3 RGB_TO_XYZ_D65 = (mat3(
0.4124564, 0.2126729, 0.0193339,
0.3575761, 0.7151522, 0.1191920,
0.1804375, 0.0721750, 0.9503041
));
const mat3 XYZ_TO_RGB_D65 = (mat3(
3.2404542, -0.9692660,  0.0556434,
-1.5371385,  1.8760108, -0.2040259,
-0.4985314,  0.0415560,  1.0572252
));

const float ZENITH_LUMA = 10.0;
const float TURBIDITY = 2.2;

// A: darkening or brightening of the horizon
// B: luminance gradient near the horizon
// C: relative intensity of the circumsolar region
// D: width of the circumsolar region
// E: relative backscattered light

const float ALT =  0.1787, AL = -1.4630;
const float BLT = -0.3554, BL =  0.4275;
const float CLT = -0.0227, CL =  5.3251;
const float DLT =  0.1206, DL = -2.5771;
const float ELT = -0.0670, EL =  0.3703;

const float AXT = -0.0193, AX = -0.2592;
const float BXT = -0.0665, BX =  0.0008;
const float CXT = -0.0004, CX =  0.2125;
const float DXT = -0.0641, DX = -0.8989;
const float EXT = -0.0033, EX =  0.0452;

const float AYT = -0.0167, AY = -0.2608;
const float BYT = -0.0950, BY =  0.0092;
const float CYT = -0.0079, CY =  0.2102;
const float DYT = -0.0441, DY = -1.6537;
const float EYT = -0.0109, EY =  0.0529;

float perez_luminance(float theta, float gamma, float a, float b, float c, float d, float e) {
    float first = 1.0 + (a * pow(E, b / cos(theta)));
    float second = 1.0 + (c * pow(E, d * gamma)) + (e * pow(cos(gamma), 2.0));
    return first * second;
}

float sky_luminance(vec3 vert_dir, vec3 sun_dir, float component) {
    vec3 up = vec3(0.0, 1.0, 0.0);
    float theta =  dot(up, vert_dir);
    float thetaS = dot(up, sun_dir);
    float gamma =  dot(vert_dir, sun_dir);

    float a = AL + (ALT * TURBIDITY);
    float b = BL + (BLT * TURBIDITY);
    float c = CL + (CLT * TURBIDITY);
    float d = DL + (DLT * TURBIDITY);
    float e = EL + (ELT * TURBIDITY);

    float f_theta_gamma = perez_luminance(theta, gamma, a, b, c, d, e);
    float f_zero_thetaS = perez_luminance(0.0, thetaS, a, b, c, d, e);
    return component * (f_theta_gamma / f_zero_thetaS);
}

vec3 sky_color(vec3 color_rgb, vec3 vert_dir, vec3 sun_dir) {
    float color_luma = dot(color_rgb, LUMA_COMPONENTS);
    vec3 color_xyz = RGB_TO_XYZ_D65 * color_rgb;
    float chroma_x = color_xyz.x / (color_xyz.x + color_xyz.y + color_xyz.z);
    float chroma_y = color_xyz.y / (color_xyz.x + color_xyz.y + color_xyz.z);

    float f_x = sky_luminance(vert_dir, sun_dir, chroma_x);
    float f_y = sky_luminance(vert_dir, sun_dir, chroma_y);
    float f_L = sky_luminance(vert_dir, sun_dir, color_luma);

    float result_xyz_x = f_x * (f_L / f_y);
    float result_xyz_z = (1.0 - f_x - f_y) * (f_L / f_y);
    vec3 result_xyz = vec3(result_xyz_x, f_L, result_xyz_z);
    return XYZ_TO_RGB_D65 * result_xyz;
}

float overcast_luminance(vec3 sun_dir) {
    vec3 up = vec3(0.0, 1.0, 0.0);
    float angle = dot(up, sun_dir);
    return ZENITH_LUMA * ((1.0 + (2.0 * cos(angle))) / 3.0);
}
