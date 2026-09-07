#include "file-queue.h"

#include <memory.h>

// Main entrypoint for this library
alert_data* driver(int day, int month, int year, unsigned int timeout, int flags) {
    struct tm time = {0};
    time.tm_mday = day;
    time.tm_mon = month;
    time.tm_year = year;

    file_queue fq;
    memset(&fq, 0, sizeof(file_queue));

    if (Init_FileQueue(&fq, &time, flags) < 0) {
        fprintf(stderr, "File queue initialization failed");
        return NULL;
    }

    alert_data *al_data = Read_FileMon(&fq, &time, timeout);

    if (fq.fp) {
        fclose(fq.fp);
    }
    return al_data;
}
