// reference:
// https://github.com/devkitPro/3ds-hbmenu/blob/master/source/loaders/rosalina.c

#include <3ds.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ENTRY_ARGBUFSIZE 0x400

static Handle hbldrHandle;

typedef struct {
  char *dst;
  u32 buf[ENTRY_ARGBUFSIZE / sizeof(u32)];
} argData_s;

static Result init(void) { return svcConnectToPort(&hbldrHandle, "hb:ldr"); }

static Result HBLDR_SetTarget(const char *path) {
  u32 pathLen = strlen(path) + 1;
  u32 *cmdbuf = getThreadCommandBuffer();

  cmdbuf[0] = IPC_MakeHeader(2, 0, 2); // 0x20002
  cmdbuf[1] = IPC_Desc_StaticBuffer(pathLen, 0);
  cmdbuf[2] = (u32)path;

  Result rc = svcSendSyncRequest(hbldrHandle);
  if (R_SUCCEEDED(rc))
    rc = cmdbuf[1];
  return rc;
}

static Result HBLDR_SetArgv(const void *buffer, u32 size) {
  u32 *cmdbuf = getThreadCommandBuffer();

  cmdbuf[0] = IPC_MakeHeader(3, 0, 2); // 0x30002
  cmdbuf[1] = IPC_Desc_StaticBuffer(size, 1);
  cmdbuf[2] = (u32)buffer;

  Result rc = svcSendSyncRequest(hbldrHandle);
  if (R_SUCCEEDED(rc))
    rc = cmdbuf[1];
  return rc;
}

static void deinit(void) { svcCloseHandle(hbldrHandle); }

Result loader_launch_file(const char *path, const char *url, const char *return_path) {
  // init
  Result rc = init();
  if (R_SUCCEEDED(rc)) {
    // args string
    argData_s args;
    memset(args.buf, '\0', sizeof(args.buf));

    // https://github.com/reswitched/RetroArch/blob/abd86058c6ca6271e993d67a1775d7f7e1aecc20/ctr/exec-3dsx/exec_3dsx.c#L40
    // append 3dsx path
    args.dst = (char *)&args.buf[1];
    strcpy(args.dst, path);
    args.dst += strlen(path) + 1;
    args.buf[0]++;

    // append args
    if (url != NULL) {
      strcpy(args.dst, url);
      args.dst += strlen(url) + 1;
      args.buf[0]++;

      if (return_path != NULL) {
        strcpy(args.dst, return_path);
        args.dst += strlen(return_path) + 1;
        args.buf[0]++;
      }
    }

    // path fix
    if (strncmp(path, "sdmc:/", 6) == 0) {
      path += 5;
    }

    // set launch path
    HBLDR_SetTarget(path);
    // set launch args
    HBLDR_SetArgv(args.buf, sizeof(args.buf));
    // deinit
    deinit();
  }

  return rc;
}
