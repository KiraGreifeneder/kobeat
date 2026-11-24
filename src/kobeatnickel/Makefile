include NickelHook/NickelHook.mk

override PKGCONF  += Qt5Widgets
override LIBRARY  := libkobeat.so
override SOURCES  += src/plugin.cc src/nickel.cc src/client.cc src/daemon.cc src/player_view.cc src/touch.cc
override MOCS     += src/plugin.h src/client.h src/player_view.h
override CFLAGS   += -Wall -Wextra -Werror
override CXXFLAGS += -Wall -Wextra -Werror -Wno-missing-field-initializers

include NickelHook/NickelHook.mk
