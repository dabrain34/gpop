# C WebSocket client for gpop-daemon

soup_dep = dependency('libsoup-3.0', version: '>= 3.0',
    required: true)

json_dep = dependency('json-glib-1.0', version: '>= 1.0',
    required: true)

# readline doesn't have pkg-config, use compiler.find_library
cc = meson.get_compiler('c')
readline_dep = cc.find_library('readline', required: true)

gpop_client_c_sources = [
  'src/main.c',
]

gpop_client_c_deps = [
  glib_dep,
  gio_dep,
  soup_dep,
  json_dep,
  readline_dep,
]

executable('gpop-client-c',
  gpop_client_c_sources,
  dependencies: gpop_client_c_deps,
  install: true,
)
