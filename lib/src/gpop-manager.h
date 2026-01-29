/*
 * GStreamer Prince of Parser
 *
 * Copyright (C) 2020 Stéphane Cerveau
 *
 * SPDX-License-Identifier: LGPL-2.1-or-later
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

#ifndef _GPOP_MANAGER_H_
#define _GPOP_MANAGER_H_

#define GPOP_TYPE_MANAGER	           (gpop_manager_get_type())
#define GPOP_MANAGER(obj)            (G_TYPE_CHECK_INSTANCE_CAST((obj),\
                                              GPOP_TYPE_MANAGER, GPOPManager))
#define GPOP_MANAGER_CLASS(klass)    (G_TYPE_CHECK_CLASS_CAST((klass),\
                                              GPOP_TYPE_MANAGER, GPOPManagerClass))
#define GPOP_MANAGER_GET_CLASS(obj)  (G_TYPE_INSTANCE_GET_CLASS ((obj),\
                                              GPOP_TYPE_MANAGER, GPOPManagerClass))
#define GPOP_IS_MANAGER(obj)         (G_TYPE_CHECK_INSTANCE_TYPE((obj),\
                                              GPOP_TYPE_MANAGER))
#define GPOP_IS_MANAGER_CLASS(klass) (G_TYPE_CHECK_CLASS_TYPE((klass),\
                                              GPOP_TYPE_MANAGER))

typedef struct _GPOPManager GPOPManager;
typedef struct _GPOPManagerClass GPOPManagerClass;

struct _GPOPManager {
  GPOPDBusInterface base;
  GList* pipelines;
};

struct _GPOPManagerClass
{
  GPOPDBusInterfaceClass base;
};

GPOPManager* gpop_manager_new (GDBusConnection* connection);
void gpop_manager_free (GPOPManager * manager);

void gpop_manager_add_pipeline (GPOPManager* manager, guint num, const gchar * parser_desc, gchar* id);
void gpop_manager_remove_pipeline (GPOPManager * manager, gchar* id);
#endif /* _GPOP_MANAGER_H_ */
