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

#ifndef _GPOP_PARSER_H_
#define _GPOP_PARSER_H_

#define GPOP_TYPE_PARSER	           (gpop_parser_get_type())
#define GPOP_PARSER(obj)            (G_TYPE_CHECK_INSTANCE_CAST((obj),\
                                              GPOP_TYPE_PARSER, GPOPParser))
#define GPOP_PARSER_CLASS(klass)    (G_TYPE_CHECK_CLASS_CAST((klass),\
                                              GPOP_TYPE_PARSER, GPOPParserClass))
#define GPOP_PARSER_GET_CLASS(obj)  (G_TYPE_INSTANCE_GET_CLASS ((obj),\
                                              GPOP_TYPE_PARSER, GPOPParserClass))
#define GPOP_IS_PARSER(obj)         (G_TYPE_CHECK_INSTANCE_TYPE((obj),\
                                              GPOP_TYPE_PARSER))
#define GPOP_IS_PARSER_CLASS(klass) (G_TYPE_CHECK_CLASS_TYPE((klass),\
                                              GPOP_TYPE_PARSER))

typedef struct _GPOPParser GPOPParser;
typedef struct _GPOPParserClass GPOPParserClass;

typedef enum {
  GPOP_PARSER_READY,
  GPOP_PARSER_PLAYING,
  GPOP_PARSER_PAUSED,
  GPOP_PARSER_EOS,
  GPOP_PARSER_ERROR,
  GPOP_PARSER_LAST,
} GPOPParserState;

struct _GPOPParserClass
{
  GObjectClass base;

  void (*state_changed) (GPOPParser * parser, GPOPParserState state);
};

GPOPParser * gpop_parser_new ();
void gpop_parser_free (GPOPParser* parser);
void gpop_parser_quit (GPOPParser * parser);

gboolean gpop_parser_play (GPOPParser *parser, const gchar * parser_desc);

gboolean gpop_parser_is_playing (GPOPParser *parser);

gboolean gpop_parser_change_state (GPOPParser * parser, GPOPParserState state);

#endif /* _GPOP_PARSER_H_ */
