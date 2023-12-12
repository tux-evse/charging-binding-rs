#include "chmgr-access.h"

#define MISSING_FUNC_FMT	"Error: Adapter does not have %s capability\n"

__s32 chmgr_smbus_access(int file, char read_write, __u8 command,
		       int size, union chmgr_smbus_data *data)
{
	struct chmgr_smbus_ioctl_data args;
	__s32 err;

	args.read_write = read_write;
	args.command = command;
	args.size = size;
	args.data = data;

	err = ioctl(file, chmgr_SMBUS, &args);
	if (err == -1)
		err = -errno;
	return err;
}

__s32 chmgr_smbus_write_byte_data(int file, __u8 command, __u8 value)
{
	union chmgr_smbus_data data;
	data.byte = value;
	return chmgr_smbus_access(file, chmgr_SMBUS_WRITE, command,
				chmgr_SMBUS_BYTE_DATA, &data);
}

__s32 chmgr_smbus_read_byte_data(int file, __u8 command)
{
	union chmgr_smbus_data data;
	int err;

	err = chmgr_smbus_access(file, chmgr_SMBUS_READ, command,
			       chmgr_SMBUS_BYTE_DATA, &data);
	if (err < 0)
		return err;

	return 0x0FF & data.byte;
}

int set_slave_addr(int file, int address, int force)
{
	/* With force, let the user read from/write to the registers
	   even when a driver is also running */
	if (ioctl(file, force ? chmgr_SLAVE_FORCE : chmgr_SLAVE, address) < 0) {
		fprintf(stderr,
			"Error: Could not set address to 0x%02x: %s\n",
			address, strerror(errno));
		return -errno;
	}

	return 0;
}

static int check_funcs_read(int file, int size, int daddress, int pec)
{
	unsigned long funcs;

	/* check adapter functionality */
	if (ioctl(file, chmgr_FUNCS, &funcs) < 0) {
		fprintf(stderr, "Error: Could not get the adapter "
			"functionality matrix: %s\n", strerror(errno));
		return -1;
	}

	switch (size) {
	case chmgr_SMBUS_BYTE:
		if (!(funcs & chmgr_FUNC_SMBUS_READ_BYTE)) {
			fprintf(stderr, MISSING_FUNC_FMT, "SMBus receive byte");
			return -1;
		}
		if (daddress >= 0
		 && !(funcs & chmgr_FUNC_SMBUS_WRITE_BYTE)) {
			fprintf(stderr, MISSING_FUNC_FMT, "SMBus send byte");
			return -1;
		}
		break;

	case chmgr_SMBUS_BYTE_DATA:
		if (!(funcs & chmgr_FUNC_SMBUS_READ_BYTE_DATA)) {
			fprintf(stderr, MISSING_FUNC_FMT, "SMBus read byte");
			return -1;
		}
		break;

	case chmgr_SMBUS_WORD_DATA:
		if (!(funcs & chmgr_FUNC_SMBUS_READ_WORD_DATA)) {
			fprintf(stderr, MISSING_FUNC_FMT, "SMBus read word");
			return -1;
		}
		break;

	case chmgr_SMBUS_BLOCK_DATA:
		if (!(funcs & chmgr_FUNC_SMBUS_READ_BLOCK_DATA)) {
			fprintf(stderr, MISSING_FUNC_FMT, "SMBus block read");
			return -1;
		}
		break;

	case chmgr_SMBUS_chmgr_BLOCK_DATA:
		if (!(funcs & chmgr_FUNC_SMBUS_READ_chmgr_BLOCK)) {
			fprintf(stderr, MISSING_FUNC_FMT, "chmgr block read");
			return -1;
		}
		break;
	}

	if (pec
	 && !(funcs & (chmgr_FUNC_SMBUS_PEC | chmgr_FUNC_chmgr))) {
		fprintf(stderr, "Warning: Adapter does "
			"not seem to support PEC\n");
	}

	return 0;
}


static int check_funcs_write(int file, int size, int pec)
{
	unsigned long funcs;

	/* check adapter functionality */
	if (ioctl(file, chmgr_FUNCS, &funcs) < 0) {
		fprintf(stderr, "Error: Could not get the adapter "
			"functionality matrix: %s\n", strerror(errno));
		return -1;
	}

	switch (size) {
	case chmgr_SMBUS_BYTE:
		if (!(funcs & chmgr_FUNC_SMBUS_WRITE_BYTE)) {
			fprintf(stderr, MISSING_FUNC_FMT, "SMBus send byte");
			return -1;
		}
		break;

	case chmgr_SMBUS_BYTE_DATA:
		if (!(funcs & chmgr_FUNC_SMBUS_WRITE_BYTE_DATA)) {
			fprintf(stderr, MISSING_FUNC_FMT, "SMBus write byte");
			return -1;
		}
		break;

	case chmgr_SMBUS_WORD_DATA:
		if (!(funcs & chmgr_FUNC_SMBUS_WRITE_WORD_DATA)) {
			fprintf(stderr, MISSING_FUNC_FMT, "SMBus write word");
			return -1;
		}
		break;

	case chmgr_SMBUS_BLOCK_DATA:
		if (!(funcs & chmgr_FUNC_SMBUS_WRITE_BLOCK_DATA)) {
			fprintf(stderr, MISSING_FUNC_FMT, "SMBus block write");
			return -1;
		}
		break;
	case chmgr_SMBUS_chmgr_BLOCK_DATA:
		if (!(funcs & chmgr_FUNC_SMBUS_WRITE_chmgr_BLOCK)) {
			fprintf(stderr, MISSING_FUNC_FMT, "chmgr block write");
			return -1;
		}
		break;
	}

	if (pec
	 && !(funcs & (chmgr_FUNC_SMBUS_PEC | chmgr_FUNC_chmgr))) {
		fprintf(stderr, "Warning: Adapter does "
			"not seem to support PEC\n");
	}

	return 0;
}

int open_chmgr_dev( char *filename)
{
	int file;
	file = open(filename, O_RDWR);
	return file;
}

int do_get(int file, int address, int daddress)
{
	int res, chmgrbus, size;
	int pec = 0;
	int length;
	unsigned char block_data[chmgr_SMBUS_BLOCK_MAX];
    int force = 0;
    size = chmgr_SMBUS_BYTE_DATA;
    length = chmgr_SMBUS_BLOCK_MAX;

	if (file < 0
	 || check_funcs_read(file, size, daddress, pec)
	 || set_slave_addr(file, address, force))
		exit(1);

	if (pec && ioctl(file, chmgr_PEC, 1) < 0) {
		fprintf(stderr, "Error: Could not set PEC: %s\n",
			strerror(errno));
		close(file);
		exit(1);
	}

	res = chmgr_smbus_read_byte_data(file, daddress);

    return res;
}

int do_set(int file, int address, int daddress, int value)
{
	char *end;
	int res, chmgrbus, size;
	int vmask = 0;
	int pec = 0;
	int opt;
	int force = 0, yes = 0, version = 0, readback = 0, all_addrs = 0;
	unsigned char block[chmgr_SMBUS_BLOCK_MAX];
	int len;

    size = chmgr_SMBUS_BYTE_DATA;

	len = 0; /* Must always initialize len since it is passed to confirm() */

	if (file < 0
	 || check_funcs_write(file, size, pec)
	 || set_slave_addr(file, address, force))
		exit(1);

	if (pec && ioctl(file, chmgr_PEC, 1) < 0) {
		fprintf(stderr, "Error: Could not set PEC: %s\n",
			strerror(errno));
		close(file);
		exit(1);
	}

	res = chmgr_smbus_write_byte_data(file, daddress, value);

	if (res < 0) {
		fprintf(stderr, "Error: Write failed\n");
		close(file);
		exit(1);
	}

	if (pec) {
		if (ioctl(file, chmgr_PEC, 0) < 0) {
			fprintf(stderr, "Error: Could not clear PEC: %s\n",
				strerror(errno));
			close(file);
			exit(1);
		}
	}

	if (res < 0) {
		printf("Warning - readback failed\n");
	} else
	if (res != value) {
		printf("Warning - data mismatch - wrote "
		       "0x%0*x, read back 0x%0*x\n",
		       size == chmgr_SMBUS_WORD_DATA ? 4 : 2, value,
		       size == chmgr_SMBUS_WORD_DATA ? 4 : 2, res);
	} else {
		printf("Value 0x%0*x written, readback matched\n",
		       size == chmgr_SMBUS_WORD_DATA ? 4 : 2, value);
	}
    return res;
}


int main(int argc, char *argv[])
{
    int chmgrbus = 0;
    int address = 0X20;
    int daddress = 0x02;
    int res;

    char filename[20];
    snprintf(filename, sizeof(filename), "/dev/chmgr-%d", chmgrbus);

    int file = open_chmgr_dev(filename);

    res=do_get(file,address, 0x00);

	printf("0x%0*x\n", 2, res);

    do_set(file, address, daddress, 0xDC);

    sleep(1);

    do_set(file, address, daddress, 0xFC);

    sleep(1);

    do_set(file, address, daddress, 0xDC);

    close(file);
	exit(0);
}
