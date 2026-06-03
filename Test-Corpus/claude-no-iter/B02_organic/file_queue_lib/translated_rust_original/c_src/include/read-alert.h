/* Copyright (C) 2015, Wazuh Inc.
 * Copyright (C) 2009 Trend Micro Inc.
 * All right reserved.
 *
 * This program is free software; you can redistribute it
 * and/or modify it under the terms of the GNU General Public
 * License (version 2) as published by the FSF - Free Software
 * Foundation
 */

#ifndef CRALERT_H
#define CRALERT_H

#include <stdio.h>

#define ALERTS_DAILY "alerts.log"

#define CRALERT_MAIL_SET    0x001
#define CRALERT_EXEC_SET    0x002
#define CRALERT_READ_ALL    0x004
#define CRALERT_READ_FAILED 0x008
#define CRALERT_FP_SET      0x010

/* File queue */
typedef struct alert_data {
    unsigned int rule;
    unsigned int level;
    char *alertid;
    char *date;
    char *location;
    char *comment;
    char *group;
    char *srcip;
    int srcport;
    char *dstip;
    int dstport;
    char *user;
    char *filename;
} alert_data;

alert_data *GetAlertData(int flag, FILE *fp) __attribute__((nonnull));
void        FreeAlertData(alert_data *al_data) __attribute__((nonnull));

#endif /* CRALERT_H */
