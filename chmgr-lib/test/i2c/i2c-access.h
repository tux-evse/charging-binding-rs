#include <sys/ioctl.h>
#include <errno.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <linux/chmgr.h>
#include <linux/chmgr-dev.h>
#include <stdlib.h>
#include <unistd.h>
#include <limits.h>
#include <dirent.h>
#include <fcntl.h>
#include <errno.h>
#include <linux/chmgr.h>
#include <linux/chmgr-dev.h>


int open_chmgr_dev(char * filename);
int do_get(int file, int address, int daddress);
int do_set(int file, int address, int daddress, int value);