/* gridlock.h — C interface to the Gridlock engine (traffic, vehicle physics, damage, city).
 *
 * Build the library from the engine workspace:
 *   cargo build --release -p gridlock-ffi
 * which produces gridlock_ffi.dll + gridlock_ffi.dll.lib (Windows), libgridlock_ffi.so (Linux)
 * or libgridlock_ffi.a (static).
 *
 * Conventions
 * - Arrays: pass a buffer and its capacity; the total count is returned (cap 0 sizes the buffer).
 * - GlMesh pointers stay valid until the next mesh call or gl_game_update.
 * - With GL_COORDS_UNREAL all positions are centimetres, left-handed, Z up (X east, Y south),
 *   quaternions are (x, y, z, w) in that frame.
 * - Mesh triangles are counter-clockwise when seen from outside (physically, in any frame).
 */
#ifndef GRIDLOCK_H
#define GRIDLOCK_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define GL_COORDS_NATIVE 0u
#define GL_COORDS_UNREAL 1u

/* Vehicle classes (GlConfig.player_class, GlCarView.class_index). */
enum {
    GL_CLASS_COMPACT = 0, GL_CLASS_SEDAN, GL_CLASS_SPORTS, GL_CLASS_MUSCLE, GL_CLASS_SUV, GL_CLASS_VAN,
    GL_CLASS_TRUCK, GL_CLASS_BUS, GL_CLASS_TAXI, GL_CLASS_POLICE, GL_CLASS_AMBULANCE
};

/* Gamepad buttons (bit index in GlPad.buttons). */
enum {
    GL_PAD_A = 0, GL_PAD_B, GL_PAD_X, GL_PAD_Y, GL_PAD_LB, GL_PAD_RB, GL_PAD_VIEW, GL_PAD_MENU,
    GL_PAD_LS, GL_PAD_RS, GL_PAD_UP, GL_PAD_DOWN, GL_PAD_LEFT, GL_PAD_RIGHT, GL_PAD_GUIDE
};

/* Event kinds. */
#define GL_EVENT_CRASH 1u
#define GL_EVENT_HORN 2u
#define GL_EVENT_SPEECH 3u
#define GL_EVENT_VIOLATION 4u
#define GL_EVENT_RAGE_START 5u
#define GL_EVENT_PART_DETACHED 6u
#define GL_EVENT_GLASS 7u
#define GL_EVENT_PROP_SNAPPED 8u
#define GL_EVENT_HYDRANT 9u
#define GL_EVENT_SHATTER 10u
#define GL_EVENT_OTHER 99u

typedef struct GlGame GlGame;

typedef struct GlConfig {
    int64_t seed;
    uint32_t traffic;
    uint32_t police;
    float start_hour;
    uint32_t player_class;
    uint32_t coords;
    uint32_t props;
} GlConfig;

typedef struct GlPad {
    float axes[6]; /* left x, left y (up +), right x, right y, left trigger, right trigger */
    uint32_t buttons;
    uint32_t connected;
} GlPad;

typedef struct GlPose {
    float pos[3];
    float rot[4]; /* x, y, z, w */
} GlPose;

typedef struct GlCarView {
    uint32_t id;
    uint32_t class_index;
    uint32_t color; /* 0xRRGGBB */
    uint32_t flags; /* bit0 player, bit1 police, bit2 physics, bit3 horn, bit4 siren, bits 8..11 wheel detached */
    GlPose pose;
    GlPose wheels[4];
    float speed;
    float rpm;
    float brake;
    uint32_t lamps; /* bit0 low, bit1 high, bit2 reverse, bit3 left, bit4 right, bit5 lightbar, bits8..15 phase, bit16/17 head broken */
    uint64_t body_revision;
} GlCarView;

typedef struct GlSignalView {
    uint32_t index;
    uint8_t ball;  /* 0 red, 1 yellow, 2 green, 3 off */
    uint8_t arrow; /* same, 255 = none */
    uint8_t ped;   /* 0 don't walk, 1 flashing, 2 walk, 3 off */
    uint8_t flags; /* bit0 ball flashing */
    float ped_countdown;
    float pole[3];
    float head[3];
    float facing[3];
} GlSignalView;

typedef struct GlPedView {
    float pos[3];
    float heading;
    float stride;
    float height;
    uint32_t shirt;
    uint32_t flags; /* bit0 walking, bit1 down */
} GlPedView;

typedef struct GlPropView {
    uint32_t kind; /* 0 lamp, 1 sign, 2 signal pole, 3 bollard, 4 hydrant, 5 trash can, 6 cone, 7 bench, 8 barrier, 9 mailbox, 10 newsstand, 11 planter */
    uint32_t broken;
    GlPose pose;
} GlPropView;

typedef struct GlEnv {
    float hour, daylight, rain, fog, wetness;
    float sun_dir[3];
} GlEnv;

typedef struct GlRumble {
    float low, high, left_trigger, right_trigger;
} GlRumble;

typedef struct GlHud {
    float speed_kmh, rpm, redline, limit_kmh, money, heat, engine_health, hour;
    float cruise_kmh; /* < 0 when off */
    uint32_t points, max_points;
    uint32_t flags; /* bit0 left, bit1 right, bit2 hazards, bit3 high beam, bit4 seatbelt, bit5 limiter, bit6 ABS, bit7 TCS, bit8 ESC, bit9 engine running */
    uint32_t headlights; /* 0 off, 1 auto, 2 on */
    char gear[8];
    char street[64];
} GlHud;

typedef struct GlEvent {
    uint32_t kind;
    uint32_t a, b; /* vehicle ids or part/prop index; 0xFFFFFFFF = none */
    float pos[3];
    float value;
    char text[96];
} GlEvent;

typedef struct GlMesh {
    const float* positions; /* xyz per vertex */
    const float* normals;   /* xyz per vertex */
    const float* uvs;       /* uv per vertex */
    const uint32_t* indices;
    uint32_t vertex_count;
    uint32_t index_count;
    uint32_t paint; /* 0 body, 1 trim, 2 glass, 3 head lamp, 4 tail lamp, 5 tyre, 6 rim */
} GlMesh;

typedef struct GlBuilding {
    float base[3];
    float size[3];
    uint32_t style; /* 0 tower, 1 block, 2 garage, 3 park, 4 plaza */
    float hue;
    uint32_t seed;
} GlBuilding;

typedef struct GlBlock {
    uint32_t point_count;
    uint32_t outer;
    float height;
    float outline[32][3];
} GlBlock;

typedef struct GlMarking {
    uint32_t kind; /* 0 white solid, 1 white dashed, 2 yellow solid, 3 stop line, 4 crosswalk */
    uint32_t point_count;
    float width;
} GlMarking;

GlGame* gl_game_create(const GlConfig* config);
void gl_game_destroy(GlGame* game);
uint32_t gl_game_update(GlGame* game, double real_dt, const GlPad* pad, const char* const* keys_down, uint32_t key_count);
uint32_t gl_game_events(GlGame* game, GlEvent* out, uint32_t cap);
uint32_t gl_game_action(GlGame* game, const char* action_id);
uint32_t gl_game_cars(GlGame* game, GlCarView* out, uint32_t cap);
uint32_t gl_game_signals(GlGame* game, GlSignalView* out, uint32_t cap);
uint32_t gl_game_peds(GlGame* game, GlPedView* out, uint32_t cap);
uint32_t gl_game_props(GlGame* game, GlPropView* out, uint32_t cap);
void gl_game_env(GlGame* game, GlEnv* out);
void gl_game_rumble(const GlGame* game, GlRumble* out);
void gl_game_hud(const GlGame* game, GlHud* out);
void gl_game_set_hour(GlGame* game, float hour);
uint32_t gl_car_model_mesh_count(GlGame* game, uint32_t class_index);
uint32_t gl_car_model_mesh(GlGame* game, uint32_t class_index, uint32_t mesh, GlMesh* out);
uint64_t gl_car_damaged_mesh(GlGame* game, uint32_t car_id, uint32_t mesh, GlMesh* out);
/* 1 = part `mesh` is loose/detached: draw it at `out` (same mesh frame) instead of with the car. */
uint32_t gl_car_part_pose(GlGame* game, uint32_t car_id, uint32_t mesh, GlPose* out);
uint32_t gl_city_buildings(GlGame* game, GlBuilding* out, uint32_t cap);
uint32_t gl_city_blocks(GlGame* game, GlBlock* out, uint32_t cap);
uint32_t gl_city_markings(GlGame* game, GlMarking* out, uint32_t cap);
uint32_t gl_city_marking_points(GlGame* game, uint32_t index, float* out_xyz, uint32_t cap_points);
uint32_t gl_city_lamp_lights(GlGame* game, float* out_xyz, uint32_t cap);
const char* gl_version(void);

#ifdef __cplusplus
}
#endif

#endif /* GRIDLOCK_H */
