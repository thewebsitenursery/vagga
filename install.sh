#!/bin/sh
/home/alinichols/=${DESTDIR:-}
/home/alinichols/bin/=${PREFIX:-/usr}

install -d ${DESTDIR}/etc/bash_completion.d
install -d ${DESTDIR}${PREFIX}/bin
install -d ${DESTDIR}${PREFIX}/lib/vagga
install -m 644 vagga-completion.sh ${DESTDIR}/etc/bash_completion.d/vagga
install -m 755 vagga ${DESTDIR}${PREFIX}/lib/vagga/vagga
install -m 755 apk ${DESTDIR}${PREFIX}/lib/vagga/apk
install -m 755 busybox ${DESTDIR}${PREFIX}/lib/vagga/busybox
install -m 755 alpine-keys.apk ${DESTDIR}${PREFIX}/lib/vagga/alpine-keys.apk
ln -snf ../lib/vagga/vagga ${DESTDIR}${PREFIX}/bin/vagga
