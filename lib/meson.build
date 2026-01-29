
src = ['src/gpop-main.c'
	   , 'src/gpop-dbus-interface.c'
	   , 'src/gpop-manager.c'
	   , 'src/gpop-pipeline.c'
	   , 'src/gpop-parser.c'
	   , 'src/gpop-client.c'
	   ]

inc = [ 'src/gpop-main.h', 'src/gpop-client.h']

soup_dep = dependency('libsoup-3.0', version: '>= 3.0', required: true)
json_dep = dependency('json-glib-1.0', version: '>= 1.0', required: true)

libgpop_dependencies = [
  glib_dep,
  gobject_dep,
  gio_dep,
  gst_dep,
  soup_dep,
  json_dep,
]

libgpop = library('libgpop'
				  , src, dependencies : libgpop_dependencies
				  , install : true)

libgpop_dep = declare_dependency(
  dependencies: libgpop_dependencies,
  sources: inc,
  include_directories: include_directories('src'),
  link_with: libgpop,
)

pkg.generate(name: 'gpop',
             description: 'Gstreamer Prince Of Parser Library',
             version: meson.project_version(),
             libraries: libgpop)