/*
 * GStreamer Prince of Parser
 *
 * Copyright (C) 2020 Stéphane Cerveau
 *
 * SPDX-License-Identifier: LGPL
 *
 * This library is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Lesser General Public
 * License as published by the Free Software Foundation
 * version 2.1 of the License.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
 * Lesser General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General Public
 * License along with this library; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301 USA
 *
 */

#include "gpop-dbus-interface.h"

G_DEFINE_TYPE (GPOPDBusInterface, gpop_dbus_interface, G_TYPE_OBJECT);
#define parent_class gpop_dbus_interface_parent_class

void
gpop_dbus_interface_handle_method_call (GDBusConnection * connection,
    const gchar * sender,
    const gchar * object_path,
    const gchar * interface_name,
    const gchar * method_name,
    GVariant * parameters,
    GDBusMethodInvocation * invocation, gpointer user_data)
{
  GPOPDBusInterface *iface = (GPOPDBusInterface *) user_data;
  GPOPDBusInterfaceClass *klass;
  klass = GPOP_DBUS_INTERFACE_GET_CLASS (iface);

  if (klass->method_call)
    klass->method_call (connection, sender, object_path, interface_name,
        method_name, parameters, invocation, user_data);
  else {
    g_dbus_method_invocation_return_value (invocation, NULL);
    g_dbus_connection_flush (connection, NULL, NULL, NULL);
  }
}

GVariant *
gpop_dbus_interface_handle_get_property (GDBusConnection * connection,
    const gchar * sender,
    const gchar * object_path,
    const gchar * interface_name,
    const gchar * property_name, GError ** error, gpointer user_data)
{
  GVariant *ret = NULL;
  GPOPDBusInterface *iface = (GPOPDBusInterface *) user_data;
  GPOPDBusInterfaceClass *klass;
  klass = GPOP_DBUS_INTERFACE_GET_CLASS (iface);

  if (klass->get_property)
    ret =
        klass->get_property (connection, sender, object_path, interface_name,
        property_name, error, user_data);

  return ret;
}

static gboolean
gpop_dbus_interface_handle_set_property (GDBusConnection * connection,
    const gchar * sender,
    const gchar * object_path,
    const gchar * interface_name,
    const gchar * property_name,
    GVariant * value, GError ** error, gpointer user_data)
{
  GPOPDBusInterface *iface = (GPOPDBusInterface *) user_data;
  GPOPDBusInterfaceClass *klass;
  klass = GPOP_DBUS_INTERFACE_GET_CLASS (iface);
  if (klass->set_property)
    return klass->set_property (connection, sender, object_path, interface_name,
        property_name, value, error, user_data);

  return *error == NULL;
}

/* for now */
static const GDBusInterfaceVTable interface_vtable = {
  gpop_dbus_interface_handle_method_call,
  gpop_dbus_interface_handle_get_property,
  gpop_dbus_interface_handle_set_property
};

/*----------------------------------------------------------------------------*
 *                            GObject interface                               *
 *----------------------------------------------------------------------------*/
static void
gpop_dbus_interface_dispose (GObject * object)
{
  GPOPDBusInterface *iface = GPOP_DBUS_INTERFACE (object);
  if (iface->object_id) {
    g_dbus_connection_unregister_object (iface->connection, iface->object_id);
    iface->object_id = 0;
  }
  iface->connection = NULL;
  g_clear_pointer (&iface->object_path, g_free);
}

static void
gpop_dbus_interface_class_init (GPOPDBusInterfaceClass * klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);
  object_class->dispose = gpop_dbus_interface_dispose;
  parent_class = g_type_class_peek_parent (klass);
}

static void
gpop_dbus_interface_init (GPOPDBusInterface * iface)
{

}

gboolean
gpop_dbus_interface_register (GPOPDBusInterface * iface,
    const gchar * object_path, const gchar * xml_introspection,
    GDBusConnection * connection)
{

  iface->introspection_data =
      g_dbus_node_info_new_for_xml (xml_introspection, NULL);

  if (!iface->introspection_data)
    return FALSE;

  iface->object_path = g_strdup (object_path);


  g_print ("object_path: %s\n", iface->object_path);


  iface->object_id = g_dbus_connection_register_object (connection, iface->object_path, iface->introspection_data->interfaces[0], &interface_vtable, iface,     /* user_data */
      NULL,                     /* user_data_free_func */
      NULL);                    /* GError** */

  if (!iface->object_id)
    return FALSE;


  iface->connection = connection;

  return TRUE;
}
