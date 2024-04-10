#include "smdh.h"
#include <3ds.h>
#include <3ds/services/cfgu.h>
#include <citro2d.h>
#include <stdlib.h>
#include <string.h>
#include <tex3ds.h>

static C2D_TextBuf g_text_buffers;
static C2D_Font g_font;
static Tex3DS_SubTexture g_icon_sub_tex = {48, 48, 0.0f, 0.75f, 0.75f, 0.0f};
static Tex3DS_SubTexture g_qrcode_sub_tex = {128, 128, 0.0f, 1.0f, 1.0f, 0.0f};

// stolen shamelessly from 3ds_hb_menu
static const u8 tile_order[] = {
    0,  1,  8,  9,  2,  3,  10, 11, 16, 17, 24, 25, 18, 19, 26, 27,
    4,  5,  12, 13, 6,  7,  14, 15, 20, 21, 28, 29, 22, 23, 30, 31,
    32, 33, 40, 41, 34, 35, 42, 43, 48, 49, 56, 57, 50, 51, 58, 59,
    36, 37, 44, 45, 38, 39, 46, 47, 52, 53, 60, 61, 54, 55, 62, 63};

typedef struct C2D_SpriteSheet_s {
  Tex3DS_Texture t3x;
  C3D_Tex tex;
} C2D_SpriteSheet_s;

void c2d_raw_init() {
  // init c2d
  C3D_Init(C3D_DEFAULT_CMDBUF_SIZE);
  C2D_Init(C2D_DEFAULT_MAX_OBJECTS);
  C2D_Prepare();
  // text buffers
  // support up to 4096 glyphs in the buffer
  g_text_buffers = C2D_TextBufNew(4096);
  // font
  g_font = C2D_FontLoadSystem(CFG_REGION_CHN);
}

void c2d_raw_fini() {
  // free text buffer
  C2D_TextBufDelete(g_text_buffers);
  // free font
  C2D_FontFree(g_font);
  // c2d fini
  C2D_Fini();
  C3D_Fini();
}

C2D_SpriteSheet_s *c2d_raw_load_sprite_sheet(const char *path) {
  return C2D_SpriteSheetLoad(path);
}

void c2d_raw_free_sprite_sheet(C2D_SpriteSheet_s *sheet) {
  if (sheet != NULL) {
    C2D_SpriteSheetFree(sheet);
  }
}

C2D_Image *c2d_raw_image_from_sheet(C2D_SpriteSheet_s *sheet, int index) {
  C2D_Image *image = malloc(sizeof(C2D_Image));
  image->tex = &sheet->tex;
  image->subtex = Tex3DS_GetSubTexture(sheet->t3x, index);
  return image;
}
void c2d_raw_free_image_from_sheet(C2D_Image *image) { free(image); }
bool c2d_raw_draw_image_at(C2D_Image *img, float x, float y, float depth,
                           float scaleX C2D_OPTIONAL(1.0f),
                           float scaleY C2D_OPTIONAL(1.0f)) {
  return C2D_DrawImageAt(*img, x, y, depth, NULL, scaleX, scaleY);
}

void c2d_raw_start_drawing() { C3D_FrameBegin(C3D_FRAME_SYNCDRAW); }
void c2d_raw_end_drawing() { C3D_FrameEnd(0); }
void c2d_raw_start_scene(C3D_RenderTarget *target) { C2D_SceneBegin(target); }
void c2d_raw_clear_scene(C3D_RenderTarget *target, u32 color) {
  C2D_TargetClear(target, color);
}

// text section
void c2d_raw_clear_text_buf() { C2D_TextBufClear(g_text_buffers); }

C2D_Text *c2d_raw_create_text(const char *str) {
  C2D_Text *text = malloc(sizeof(C2D_Text));
  C2D_TextFontParse(text, g_font, g_text_buffers, str);
  C2D_TextOptimize(text);
  return text;
}

void c2d_raw_free_text(C2D_Text *text) { free(text); }

void c2d_raw_draw_text(const C2D_Text *text, float x, float y, float z,
                       float scaleX, float scaleY, u32 color,
                       const float *max_width) {

  if (max_width != NULL) {
    C2D_DrawText(text, C2D_WithColor | C2D_WordWrap, x, y, z, scaleX, scaleY,
                 color, *max_width);
  } else {
    C2D_DrawText(text, C2D_WithColor, x, y, z, scaleX, scaleY, color);
  }
}

bool c2d_drawrectsolid(float x, float y, float z, float w, float h, u32 clr) {
  return C2D_DrawRectangle(x, y, z, w, h, clr, clr, clr, clr);
}

void c2d_raw_free_image(C2D_Image *image) {
  if (image != NULL) {
    if (image->tex != NULL) {
      C3D_TexDelete(image->tex);
      free(image->tex);
    }
    free(image);
  }
}

C2D_Image *c2d_raw_load_icon_from_buffer(u16 *icon) {
  C3D_Tex *ret = malloc(sizeof(C3D_Tex));
  C3D_TexSetFilter(ret, GPU_LINEAR, GPU_LINEAR);
  /*
   *
   * 64x64 is the minimum size for a texture on the 3DS
   * 48x48 is the size of the icon in the SMDH
   *
   *   |----------------
   *   |   64*64       |
   *   |------------   |
   *   |           |   |
   *   |   48*48   |   |
   *   |           |   |
   *   |----------------
   * (0, 0)
   */
  if (C3D_TexInit(ret, 64, 64, GPU_RGB565)) // GPU can't use below 64x64
  {
    uint16_t *tex = (uint16_t *)ret->data + (16 * 64);
    for (unsigned y = 0; y < 48; y += 8, icon += 48 * 8, tex += 64 * 8)
      memcpy(tex, icon, sizeof(uint16_t) * 48 * 8);
  }
  C2D_Image *image = malloc(sizeof(C2D_Image));
  image->tex = ret;
  image->subtex = &g_icon_sub_tex;
  return image;
}

C2D_Image *c2d_raw_load_qrcode_from_buffer(const u8 *icon) {
  u8 large_icon_data[128 * 128];
  u8 *large_icon = large_icon_data;
  u32 x, y, xx, yy, k;
  u32 n = 0;

  // https://devkitpro.org/viewtopic.php?f=39&t=9219
  // https://github.com/devkitPro/3dstools/blob/master/src/smdhtool.cpp#L347
  for (y = 0; y < 128; y += 8) {
    for (x = 0; x < 128; x += 8) {
      for (k = 0; k < 8 * 8; k++) {
        xx = (tile_order[k] & 0x7);
        yy = (tile_order[k] >> 3);
        large_icon[n++] = icon[1 * (128 * (y + yy) + (x + xx))];
      }
    }
  }

  C3D_Tex *ret = malloc(sizeof(C3D_Tex));
  C3D_TexSetFilter(ret, GPU_LINEAR, GPU_LINEAR);
  if (C3D_TexInit(ret, 128, 128, GPU_L8)) {
    u8 *tex = (u8 *)ret->data;
    for (u32 y = 0; y < 128; y += 8, large_icon += 128 * 8, tex += 128 * 8)
      memcpy(tex, large_icon, sizeof(u8) * 128 * 8);
  }

  C2D_Image *image = malloc(sizeof(C2D_Image));
  image->tex = ret;
  image->subtex = &g_qrcode_sub_tex;
  return image;
}
