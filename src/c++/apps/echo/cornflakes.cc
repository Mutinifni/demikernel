// Copyright (c) Microsoft Corporation
// Licensed under the MIT license.

#include "cornflakes.hh"
#include "stress_cornflakes.hh"
#include "message.hh"
#include <sys/mman.h>
#include <dmtr/sga.h>
#include <dmtr/libos/mem.h>
//#include <rte_malloc.h>

cornflakes_echo::cornflakes_echo(uint32_t field_size, string message_type) :
    echo_message(echo_message::library::CORNFLAKES, field_size, message_type),
    mmap_addr(NULL),
    mmap_len(0),
    mmap_available_len(0),
    recv_payload(NULL)
{
}

void cornflakes_echo::init_ext_mem() {
    /*void *addr = rte_malloc("Serialization_Memory", 40960, 0);
    if (addr == NULL) {
        printf("Rte malloc failed\n");
        exit(1);
    }*/
    void *addr = mmap(NULL, 4096 * 1000, PROT_READ | PROT_WRITE, MAP_ANONYMOUS | MAP_SHARED, -1, 0);
    if (addr == MAP_FAILED) {
        printf("Failed to mmap memory for cornflakes_echo.\n");
        exit(1);
    }
    mmap_addr = addr;
    mmap_len = 4096 * 1000;
    mmap_available_len = 4096 * 1000;
    memset(mmap_addr, 'm', mmap_len);

}

cornflakes_echo::~cornflakes_echo() {
    munmap(mmap_addr, mmap_len);
}

void * cornflakes_echo::get_mmap_addr() {
    return mmap_addr;
}

size_t cornflakes_echo::get_mmap_len() {
    return mmap_len;
}

void cornflakes_echo::set_mmap_available_len(size_t len) {
    mmap_available_len = len;
}

void cornflakes_echo::serialize_message(dmtr_sgarray_t &sga, void *context) {
    cornflakes::GetMessageCF *getMsg = (cornflakes::GetMessageCF *)context;
    getMsg->set_key((char *)mmap_addr, field_size());
    getMsg->serialize(sga);
}

void cornflakes_echo::deserialize_message(dmtr_sgarray_t &sga) {
    cornflakes::GetMessageCF getMsg;
    getMsg.deserialize(sga);
    recv_payload = (void *)(getMsg.get_key().ptr);
}



