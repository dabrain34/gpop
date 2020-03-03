project('gpop', 'c', version : '1.0.0', license : 'LGPL')

cc = meson.get_compiler('c')

# Dependencies
glib_req_version = '>= 2.44.0'
gst_req_version = '>= 1.16.0'

glib_dep = dependency('glib-2.0', version: glib_req_version,
    fallback: ['glib', 'libglib_dep'])
gio_dep = [dependency('gio-2.0', version: glib_req_version,
                  fallback: ['glib', 'libgio_dep']),
           dependency('gio-unix-2.0', version: glib_req_version,
                  fallback: ['glib', 'libgio_dep'])]
gobject_dep = dependency('gobject-2.0', version: glib_req_version,
    fallback: ['glib', 'libgobject_dep'])

gst_dep = dependency('gstreamer-1.0', version: gst_req_version,
    fallback : ['gstreamer', 'gst_dep'])

root_inc = include_directories('.')

subdir('lib')
subdir('daemon')
