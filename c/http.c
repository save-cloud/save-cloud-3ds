#include <curl/curl.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// define u64
typedef unsigned long long u64;
/// Checks whether a result code indicates success.
#define R_SUCCEEDED(res) ((res) == 0)
/// Checks whether a result code indicates failure.
#define R_FAILED(res) ((res) != 0)
/// download buffer size
#define DOWNLOAD_BUFFER_SIZE 512 * 1024

static bool is_file_exist(const char *path) {
  FILE *f = fopen(path, "r");
  if (f == NULL) {
    return false;
  } else {
    fclose(f);
    return true;
  }
}

static char *read_whole_file(const char *path, size_t *size) {
  FILE *f = fopen(path, "rb");
  fseek(f, 0, SEEK_END);
  (*size) = ftell(f);
  fseek(f, 0, SEEK_SET); /* same as rewind(f); */

  char *content = malloc((*size) + 1);
  fread(content, (*size), 1, f);
  fclose(f);

  content[(*size)] = 0;

  return content;
}

typedef struct HttpData {
  FILE *download_file_fd;
  unsigned char *download_buffer;
  size_t current_buf_size;
  size_t size;
  char *response;
  size_t header_size;
  char *header;
} HttpData;

typedef struct HttpResponse {
  long status;
  const char *message;
  size_t size;
  size_t header_size;
  char *response;
  char *header;
} HttpResponse;

static size_t _curl_cb_header(void *data, size_t size, size_t nmemb,
                              void *client_data_ptr) {
  HttpData *mem = (HttpData *)client_data_ptr;

  size_t realsize = size * nmemb;
  char *ptr = realloc(mem->header, mem->header_size + realsize + 1);
  if (ptr == NULL)
    return 0; /* out of memory! */

  mem->header = ptr;
  memcpy(&(mem->header[mem->header_size]), data, realsize);
  mem->header_size += realsize;
  mem->header[mem->header_size] = 0;
  return realsize;
}

static size_t _curl_cb(void *data, size_t size, size_t nmemb,
                       void *client_data_ptr) {
  HttpData *mem = (HttpData *)client_data_ptr;

  if (mem->download_file_fd != NULL) {
    size_t total_size = size * nmemb;
    size_t remain_size = total_size;
    while (remain_size > 0) {
      size_t write_size =
          remain_size > DOWNLOAD_BUFFER_SIZE - mem->current_buf_size
              ? DOWNLOAD_BUFFER_SIZE - mem->current_buf_size
              : remain_size;
      memcpy(&(mem->download_buffer[mem->current_buf_size]),
             data + total_size - remain_size, write_size);
      mem->current_buf_size += write_size;
      remain_size -= write_size;
      if (mem->current_buf_size == DOWNLOAD_BUFFER_SIZE) {
        size_t res = fwrite(mem->download_buffer, 1, DOWNLOAD_BUFFER_SIZE,
                            mem->download_file_fd);
        mem->current_buf_size = 0;
        if (res != DOWNLOAD_BUFFER_SIZE) {
          return res;
        }
      }
    }
    return total_size;
  } else {
    size_t realsize = size * nmemb;
    char *ptr = realloc(mem->response, mem->size + realsize + 1);
    if (ptr == NULL)
      return 0; /* out of memory! */

    mem->response = ptr;
    memcpy(&(mem->response[mem->size]), data, realsize);
    mem->size += realsize;
    mem->response[mem->size] = 0;
    return realsize;
  }
}

HttpResponse *http_request(
    const char *method, const char *url, const char *user_agent,
    const char *body, const char *file_to_upload_name,
    const char *file_to_upload_path, const char *data_to_upload,
    size_t data_to_upload_len, const char *download_file_path, bool ssl_verify,
    int (*progress_cb)(void *clientp, long long dltotal, long long dlnow,
                       long long ultotal, long long ulnow),
    void *clientp, int is_follow) {
  CURL *curl;
  CURLcode res;
  curl_mime *form = NULL;
  curl_mimepart *field = NULL;

  // data ptr;
  HttpData *data = malloc(sizeof(HttpData));
  data->download_buffer = NULL;
  data->current_buf_size = 0;
  data->download_file_fd = NULL;
  data->size = 0;
  data->header_size = 0;
  data->response = NULL;
  data->header = NULL;
  // res
  HttpResponse *response = malloc(sizeof(HttpResponse));
  response->status = -1;
  response->message = "";
  response->size = 0;
  response->response = NULL;
  response->header = NULL;

  if (download_file_path != NULL) {
    data->download_file_fd = fopen(download_file_path, "wb");
    if (!data->download_file_fd) {
      response->status = -2;
      response->message = "创建文件失败";
      free(data);
      return NULL;
    }
    data->download_buffer = malloc(DOWNLOAD_BUFFER_SIZE);
  }

  curl = curl_easy_init();

  if (curl) {
    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_BUFFERSIZE, 128 * 1024);
    if (file_to_upload_name != NULL) {
      form = curl_mime_init(curl);
      field = curl_mime_addpart(form);
      curl_mime_name(field, file_to_upload_name);
      if (data_to_upload != NULL) {
        curl_mime_filename(field, file_to_upload_path);
        curl_mime_data(field, data_to_upload, data_to_upload_len);
      } else {
        curl_mime_filedata(field, file_to_upload_path);
      }
      curl_easy_setopt(curl, CURLOPT_MIMEPOST, form);
    } else if (strcmp(method, "POST") == 0) {
      curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, "POST");
      if (body != NULL) {
        curl_easy_setopt(curl, CURLOPT_COPYPOSTFIELDS, body);
      }
    }
    if (progress_cb != NULL) {
      curl_easy_setopt(curl, CURLOPT_NOPROGRESS, 0L);
      curl_easy_setopt(curl, CURLOPT_XFERINFODATA, clientp);
      curl_easy_setopt(curl, CURLOPT_XFERINFOFUNCTION, progress_cb);
    }

    // ssl
    if (ssl_verify) {
      curl_easy_setopt(curl, CURLOPT_CAINFO, "/config/ssl/cacert.pem");
    } else {
      curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 0L);
      curl_easy_setopt(curl, CURLOPT_SSL_VERIFYHOST, 0L);
    }
    if (user_agent != NULL) {
      curl_easy_setopt(curl, CURLOPT_USERAGENT, user_agent);
    }
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, _curl_cb);
    curl_easy_setopt(curl, CURLOPT_HEADERFUNCTION, _curl_cb_header);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, data);
    curl_easy_setopt(curl, CURLOPT_HEADERDATA, data);
    if (is_follow) {
      curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);
    }
    /* curl_easy_setopt(curl, CURLOPT_VERBOSE, 1L); */

    res = curl_easy_perform(curl);

    if (res != CURLE_OK) {
      response->status = res;
      response->message = curl_easy_strerror(res);
    } else {
      curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &response->status);
    }

    curl_easy_cleanup(curl);

    if (form != NULL) {
      curl_mime_free(form);
    }
  } else {
    response->status = -3;
    response->message = "curl_easy_init 失败";
  }

  if (data->download_file_fd != NULL) {
    if (data->current_buf_size > 0) {
      fwrite(data->download_buffer, 1, data->current_buf_size,
             data->download_file_fd);
      data->current_buf_size = 0;
    }
    free(data->download_buffer);
    fclose(data->download_file_fd);
    data->download_file_fd = NULL;
  }

  if (response->status != 200 && download_file_path != NULL &&
      is_file_exist(download_file_path)) {
    if (data->response != NULL) {
      free(data->response);
    }
    data->response = read_whole_file(download_file_path, &data->size);
    remove(download_file_path);
  }

  response->size = data->size;
  response->response = data->response;
  response->header_size = data->header_size;
  response->header = data->header;
  free(data);

  return response;
}

void http_init() { curl_global_init(CURL_GLOBAL_ALL); }

void http_exit() { curl_global_cleanup(); }

void http_free_response(HttpResponse *res) {
  if (res->response != NULL) {
    free(res->response);
  }
  if (res->header != NULL) {
    free(res->header);
  }
  free(res);
}
