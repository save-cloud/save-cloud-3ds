#include "smdh.h"
#include <3ds.h>
#include <stdlib.h>
#include <string.h>

#define IS_N3DS                                                                \
  (OS_KernelConfig->app_memtype >= 6) // APPMEMTYPE. Hacky but doesn't use APT

typedef struct Title {
  u64 id;
  FS_MediaType media_type;
  char *product_code;
  char *desc_short;
  char *desc_long;
  u8 has_user_save;
  u8 has_ext_data;
  u8 has_sys_data;
  u8 has_boss_data;
  u8 has_shared_data;
} Title;

static inline u32 highId(u64 mId) { return (u32)(mId >> 32); }

static inline u32 lowId(u64 mId) { return (u32)mId; }

smdh_s *pl_get_smdh(u64 id, u8 media) {
  u32 low = lowId(id);
  u32 high = highId(id);
  Handle fileHandle;

  u32 archPath[] = {low, high, media, 0x0};
  static const u32 filePath[] = {0x0, 0x0, 0x2, 0x6E6F6369, 0x0};
  smdh_s *smdh = malloc(sizeof(smdh_s));

  FS_Path binArchPath = {PATH_BINARY, 0x10, archPath};
  FS_Path binFilePath = {PATH_BINARY, 0x14, filePath};

  Result res =
      FSUSER_OpenFileDirectly(&fileHandle, ARCHIVE_SAVEDATA_AND_CONTENT,
                              binArchPath, binFilePath, FS_OPEN_READ, 0);
  if (R_SUCCEEDED(res)) {
    u32 read;
    FSFILE_Read(fileHandle, &read, 0, smdh, sizeof(smdh_s));
  } else {
    free(smdh);
    smdh = NULL;
  }

  FSFILE_Close(fileHandle);
  return smdh;
}

void pl_free(smdh_s *ptr) {
  if (ptr != NULL) {
    free(ptr);
  }
}

char *pl_get_smdh_short_desc(smdh_s *smdh) {
  char *desc_short = malloc(2 * 0x40);
  memset(desc_short, 0, 2 * 0x40);
  utf16_to_utf8((uint8_t *)desc_short,
                smdh->applicationTitles[1].shortDescription, 0x40 * 2);
  return desc_short;
}

/* u8 pl_get_wifi_strength() { return osGetWifiStrength(); } */

const u16 *pl_get_icon_buffer_from_smdh(smdh_s *smdh) {
  return smdh->bigIconData;
}

bool pl_is_n3ds() { return IS_N3DS; }

Result pl_commit_data(const FS_ArchiveID arch_id, const FS_Archive arch) {
  Result res = 0;
  if (arch_id != ARCHIVE_EXTDATA && arch_id != ARCHIVE_BOSS_EXTDATA) {
    res = FSUSER_ControlArchive(arch, ARCHIVE_ACTION_COMMIT_SAVE_DATA, NULL, 0,
                                NULL, 0);
  }
  return res;
}

Result pl_delete_sv(const FS_ArchiveID arch, const u32 unique_id) {
  Result res = 0;
  if (arch != ARCHIVE_EXTDATA && arch != ARCHIVE_BOSS_EXTDATA) {
    u64 in = ((u64)SECUREVALUE_SLOT_SD << 32) | (unique_id << 8);
    u8 out;

    res = FSUSER_ControlSecureSave(SECURESAVE_ACTION_DELETE, &in, 8, &out, 1);
  }
  return res;
}

Result pl_open_title(const u64 title_id, const FS_MediaType media,
                     const char *path) {
  Result res = 0;
  if (R_SUCCEEDED(res = APT_PrepareToDoApplicationJump(0, title_id, media))) {
    u8 param[0x300];
    memset(param, 0, sizeof(param));
    strncpy((char *)param, path, sizeof(param));
    u8 hmac[0x20];

    res = APT_DoApplicationJump(param, sizeof(param), hmac);
  }
  return res;
}

bool pl_env_is_homebrew() { return envIsHomebrew(); }

Result pl_get_storage_info(u64 *free, u64 *total) {
  FS_SystemMediaType mediatype = SYSTEM_MEDIATYPE_SD;
  FS_ArchiveResource resource = {0};
  Result ret = FSUSER_GetArchiveResource(&resource, mediatype);

  *free = (u64)resource.freeClusters * (u64)resource.clusterSize;
  *total = (u64)resource.totalClusters * (u64)resource.clusterSize;
  return ret;
}
