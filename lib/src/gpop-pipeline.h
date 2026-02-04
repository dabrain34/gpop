/*
 * GStreamer Prince of Parser
 *
 * Copyright (C) 2020 Stéphane Cerveau <scerveau@gmail.com>
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

#ifndef _GPOP_PIPELINE_H_
#define _GPOP_PIPELINE_H_

#define GPOP_TYPE_PIPELINE	           (gpop_pipeline_get_type())
#define GPOP_PIPELINE(obj)            (G_TYPE_CHECK_INSTANCE_CAST((obj),\
                                              GPOP_TYPE_PIPELINE, GPOPPipeline))
#define GPOP_PIPELINE_CLASS(klass)    (G_TYPE_CHECK_CLASS_CAST((klass),\
                                              GPOP_TYPE_PIPELINE, GPOPPipelineClass))
#define GPOP_PIPELINE_GET_CLASS(obj)  (G_TYPE_INSTANCE_GET_CLASS ((obj),\
                                              GPOP_TYPE_PIPELINE, GPOPPipelineClass))
#define GPOP_IS_PIPELINE(obj)         (G_TYPE_CHECK_INSTANCE_TYPE((obj),\
                                              GPOP_TYPE_PIPELINE))
#define GPOP_IS_PIPELINE_CLASS(klass) (G_TYPE_CHECK_CLASS_TYPE((klass),\
                                              GPOP_TYPE_PIPELINE))

typedef struct _GPOPPipeline GPOPPipeline;
typedef struct _GPOPPipelineClass GPOPPipelineClass;

struct _GPOPPipeline
{
  GPOPDBusInterface base;
  GPOPParser * parser;
  GPOPManager *manager;
  guint num;
  gchar * id;
  gchar * parser_desc;
};

struct _GPOPPipelineClass
{
  GPOPDBusInterfaceClass base;
};

GPOPPipeline * gpop_pipeline_new (GPOPManager* manager, GDBusConnection* connection, guint num);
void gpop_pipeline_free (GPOPPipeline* pipeline);
gboolean gpop_pipeline_set_state (GPOPPipeline* pipeline, GPOPParserState state);
gboolean gpop_pipeline_set_parser_desc (GPOPPipeline* pipeline, const gchar * parser_desc);

#endif /* _GPOP_PIPELINE_H_ */
