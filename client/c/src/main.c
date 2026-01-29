/*
 * GStreamer Prince of Parser - C WebSocket Client
 *
 * Copyright (C) 2020-2024 Stephane Cerveau
 *
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, version 3 of the License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

#include <glib.h>
#include <gio/gio.h>
#include <libsoup/soup.h>
#include <json-glib/json-glib.h>
#include <string.h>
#include <stdio.h>

#define DEFAULT_URL "ws://127.0.0.1:8444"

typedef struct {
    GMainLoop *loop;
    SoupSession *session;
    SoupWebsocketConnection *ws;
    GIOChannel *stdin_channel;
    guint stdin_watch_id;
    gchar *url;
    gboolean connected;
} GpopClient;

static GpopClient *client = NULL;

static gchar *
generate_uuid (void)
{
    return g_uuid_string_random ();
}

static gchar *
strip_quotes (const gchar *str)
{
    gsize len = strlen (str);
    if (len >= 2) {
        if ((str[0] == '"' && str[len - 1] == '"') ||
            (str[0] == '\'' && str[len - 1] == '\'')) {
            return g_strndup (str + 1, len - 2);
        }
    }
    return g_strdup (str);
}

static void
print_help (void)
{
    g_print ("\nAvailable commands:\n");
    g_print ("  list                      - List all pipelines\n");
    g_print ("  create <description>      - Create a new pipeline\n");
    g_print ("  update <id> <description> - Update pipeline description\n");
    g_print ("  remove <id>               - Remove a pipeline\n");
    g_print ("  info <id>                 - Get pipeline info\n");
    g_print ("  play <id>                 - Play a pipeline\n");
    g_print ("  pause <id>                - Pause a pipeline\n");
    g_print ("  stop <id>                 - Stop a pipeline\n");
    g_print ("  state <id> <state>        - Set pipeline state\n");
    g_print ("  dot <id> [details]        - Get DOT graph (details: media, caps, states, all)\n");
    g_print ("  position [id]             - Get pipeline position/duration (default: 0)\n");
    g_print ("  help                      - Show this help\n");
    g_print ("  quit                      - Exit\n");
    g_print ("\n");
}

static gchar *
json_to_pretty_string (JsonNode *node)
{
    JsonGenerator *gen = json_generator_new ();
    json_generator_set_pretty (gen, TRUE);
    json_generator_set_indent (gen, 2);
    json_generator_set_root (gen, node);
    gchar *str = json_generator_to_data (gen, NULL);
    g_object_unref (gen);
    return str;
}

static gchar *
create_request (const gchar *method, JsonObject *params)
{
    JsonBuilder *builder = json_builder_new ();
    gchar *uuid = generate_uuid ();

    json_builder_begin_object (builder);

    json_builder_set_member_name (builder, "id");
    json_builder_add_string_value (builder, uuid);

    json_builder_set_member_name (builder, "method");
    json_builder_add_string_value (builder, method);

    json_builder_set_member_name (builder, "params");
    if (params) {
        JsonNode *params_node = json_node_new (JSON_NODE_OBJECT);
        json_node_set_object (params_node, params);
        json_builder_add_value (builder, params_node);
    } else {
        json_builder_begin_object (builder);
        json_builder_end_object (builder);
    }

    json_builder_end_object (builder);

    JsonGenerator *gen = json_generator_new ();
    JsonNode *root = json_builder_get_root (builder);
    json_generator_set_root (gen, root);
    gchar *json_str = json_generator_to_data (gen, NULL);

    g_object_unref (gen);
    g_object_unref (builder);
    g_free (uuid);

    return json_str;
}

static void
handle_event (JsonObject *root)
{
    const gchar *event_type = json_object_get_string_member (root, "event");
    JsonNode *data_node = json_object_get_member (root, "data");

    gchar *data_str = json_to_pretty_string (data_node);
    g_print ("\n[EVENT] %s: %s\n> ", event_type, data_str);
    fflush (stdout);
    g_free (data_str);
}

static void
handle_response (JsonObject *root)
{
    const gchar *id = json_object_get_string_member (root, "id");

    if (json_object_has_member (root, "error")) {
        JsonObject *error = json_object_get_object_member (root, "error");
        gint64 code = json_object_get_int_member (error, "code");
        const gchar *message = json_object_get_string_member (error, "message");
        g_print ("\n[ERROR] id=%s: %s (code: %" G_GINT64_FORMAT ")\n> ", id, message, code);
    } else if (json_object_has_member (root, "result")) {
        JsonNode *result_node = json_object_get_member (root, "result");
        gchar *result_str = json_to_pretty_string (result_node);
        g_print ("\n[RESPONSE] id=%s: %s\n> ", id, result_str);
        g_free (result_str);
    }
    fflush (stdout);
}

static void
process_message (const gchar *text)
{
    JsonParser *parser = json_parser_new ();
    GError *error = NULL;

    if (!json_parser_load_from_data (parser, text, -1, &error)) {
        g_print ("\n[RAW] %s\n> ", text);
        fflush (stdout);
        g_error_free (error);
        g_object_unref (parser);
        return;
    }

    JsonNode *root_node = json_parser_get_root (parser);
    JsonObject *root = json_node_get_object (root_node);

    if (json_object_has_member (root, "event")) {
        handle_event (root);
    } else if (json_object_has_member (root, "id")) {
        handle_response (root);
    } else {
        g_print ("\n[RAW] %s\n> ", text);
        fflush (stdout);
    }

    g_object_unref (parser);
}

static void
on_websocket_message (SoupWebsocketConnection *ws,
                      SoupWebsocketDataType type,
                      GBytes *message,
                      gpointer user_data)
{
    (void) ws;
    (void) user_data;

    if (type == SOUP_WEBSOCKET_DATA_TEXT) {
        gsize len;
        const gchar *data = g_bytes_get_data (message, &len);
        gchar *text = g_strndup (data, len);
        process_message (text);
        g_free (text);
    }
}

static void
on_websocket_closed (SoupWebsocketConnection *ws, gpointer user_data)
{
    (void) ws;
    GpopClient *c = (GpopClient *) user_data;

    g_print ("\nConnection closed\n");
    c->connected = FALSE;
    g_main_loop_quit (c->loop);
}

static void
on_websocket_error (SoupWebsocketConnection *ws,
                    GError *error,
                    gpointer user_data)
{
    (void) ws;
    (void) user_data;

    g_printerr ("\nWebSocket error: %s\n", error->message);
}

static void
send_request (GpopClient *c, const gchar *method, JsonObject *params)
{
    if (!c->connected || !c->ws) {
        g_print ("Not connected\n");
        return;
    }

    gchar *json_str = create_request (method, params);
    g_print ("Sending: %s\n", json_str);
    soup_websocket_connection_send_text (c->ws, json_str);
    g_free (json_str);
}

static JsonObject *
build_pipeline_id_params (const gchar *id)
{
    JsonObject *params = json_object_new ();
    json_object_set_string_member (params, "pipeline_id", id);
    return params;
}

static JsonObject *
build_create_params (const gchar *description)
{
    JsonObject *params = json_object_new ();
    json_object_set_string_member (params, "description", description);
    return params;
}

static JsonObject *
build_update_params (const gchar *id, const gchar *description)
{
    JsonObject *params = json_object_new ();
    json_object_set_string_member (params, "pipeline_id", id);
    json_object_set_string_member (params, "description", description);
    return params;
}

static JsonObject *
build_set_state_params (const gchar *id, const gchar *state)
{
    JsonObject *params = json_object_new ();
    json_object_set_string_member (params, "pipeline_id", id);
    json_object_set_string_member (params, "state", state);
    return params;
}

static JsonObject *
build_snapshot_params (const gchar *id, const gchar *details)
{
    JsonObject *params = json_object_new ();
    json_object_set_string_member (params, "pipeline_id", id);
    if (details) {
        json_object_set_string_member (params, "details", details);
    } else {
        json_object_set_null_member (params, "details");
    }
    return params;
}

static JsonObject *
build_position_params (const gchar *id)
{
    JsonObject *params = json_object_new ();
    if (id) {
        json_object_set_string_member (params, "pipeline_id", id);
    } else {
        json_object_set_null_member (params, "pipeline_id");
    }
    return params;
}

static void
process_command (GpopClient *c, const gchar *line)
{
    gchar **parts = g_strsplit (line, " ", -1);
    gint argc = g_strv_length (parts);

    if (argc == 0 || strlen (parts[0]) == 0) {
        g_strfreev (parts);
        return;
    }

    const gchar *cmd = parts[0];

    if (g_strcmp0 (cmd, "list") == 0) {
        send_request (c, "list_pipelines", NULL);
    }
    else if (g_strcmp0 (cmd, "create") == 0 && argc > 1) {
        gchar *joined = g_strjoinv (" ", parts + 1);
        gchar *description = strip_quotes (joined);
        JsonObject *params = build_create_params (description);
        send_request (c, "create_pipeline", params);
        json_object_unref (params);
        g_free (description);
        g_free (joined);
    }
    else if (g_strcmp0 (cmd, "update") == 0 && argc > 2) {
        gchar *joined = g_strjoinv (" ", parts + 2);
        gchar *description = strip_quotes (joined);
        JsonObject *params = build_update_params (parts[1], description);
        send_request (c, "update_pipeline", params);
        json_object_unref (params);
        g_free (description);
        g_free (joined);
    }
    else if (g_strcmp0 (cmd, "remove") == 0 && argc == 2) {
        JsonObject *params = build_pipeline_id_params (parts[1]);
        send_request (c, "remove_pipeline", params);
        json_object_unref (params);
    }
    else if (g_strcmp0 (cmd, "info") == 0 && argc == 2) {
        JsonObject *params = build_pipeline_id_params (parts[1]);
        send_request (c, "get_pipeline_info", params);
        json_object_unref (params);
    }
    else if (g_strcmp0 (cmd, "play") == 0 && argc == 2) {
        JsonObject *params = build_pipeline_id_params (parts[1]);
        send_request (c, "play", params);
        json_object_unref (params);
    }
    else if (g_strcmp0 (cmd, "pause") == 0 && argc == 2) {
        JsonObject *params = build_pipeline_id_params (parts[1]);
        send_request (c, "pause", params);
        json_object_unref (params);
    }
    else if (g_strcmp0 (cmd, "stop") == 0 && argc == 2) {
        JsonObject *params = build_pipeline_id_params (parts[1]);
        send_request (c, "stop", params);
        json_object_unref (params);
    }
    else if (g_strcmp0 (cmd, "state") == 0 && argc == 3) {
        JsonObject *params = build_set_state_params (parts[1], parts[2]);
        send_request (c, "set_state", params);
        json_object_unref (params);
    }
    else if (g_strcmp0 (cmd, "dot") == 0 && argc >= 2) {
        const gchar *details = (argc > 2) ? parts[2] : NULL;
        JsonObject *params = build_snapshot_params (parts[1], details);
        send_request (c, "snapshot", params);
        json_object_unref (params);
    }
    else if (g_strcmp0 (cmd, "position") == 0) {
        const gchar *id = (argc > 1) ? parts[1] : NULL;
        JsonObject *params = build_position_params (id);
        send_request (c, "get_position", params);
        json_object_unref (params);
    }
    else if (g_strcmp0 (cmd, "help") == 0) {
        print_help ();
    }
    else if (g_strcmp0 (cmd, "quit") == 0 || g_strcmp0 (cmd, "exit") == 0) {
        g_print ("Goodbye!\n");
        g_main_loop_quit (c->loop);
    }
    else {
        g_print ("Unknown command or missing arguments. Type 'help' for available commands.\n");
    }

    g_strfreev (parts);
}

static gboolean
on_stdin_ready (GIOChannel *source,
                GIOCondition condition,
                gpointer user_data)
{
    GpopClient *c = (GpopClient *) user_data;

    if (condition & G_IO_HUP) {
        g_print ("\nGoodbye!\n");
        g_main_loop_quit (c->loop);
        return FALSE;
    }

    gchar *line = NULL;
    gsize len;
    GError *error = NULL;
    GIOStatus status = g_io_channel_read_line (source, &line, &len, NULL, &error);

    if (status == G_IO_STATUS_NORMAL && line) {
        g_strstrip (line);
        if (strlen (line) > 0) {
            process_command (c, line);
        }
        g_print ("> ");
        fflush (stdout);
        g_free (line);
    } else if (status == G_IO_STATUS_EOF) {
        g_print ("\nGoodbye!\n");
        g_main_loop_quit (c->loop);
        return FALSE;
    } else if (error) {
        g_printerr ("Error reading stdin: %s\n", error->message);
        g_error_free (error);
    }

    return TRUE;
}

static void
setup_stdin (GpopClient *c)
{
#ifdef G_OS_UNIX
    c->stdin_channel = g_io_channel_unix_new (fileno (stdin));
#else
    c->stdin_channel = g_io_channel_win32_new_fd (fileno (stdin));
#endif

    g_io_channel_set_encoding (c->stdin_channel, NULL, NULL);
    g_io_channel_set_buffered (c->stdin_channel, TRUE);

    c->stdin_watch_id = g_io_add_watch (c->stdin_channel,
                                         G_IO_IN | G_IO_HUP,
                                         on_stdin_ready,
                                         c);
}

static void
on_websocket_connected (GObject *source,
                        GAsyncResult *result,
                        gpointer user_data)
{
    GpopClient *c = (GpopClient *) user_data;
    GError *error = NULL;

    c->ws = soup_session_websocket_connect_finish (SOUP_SESSION (source),
                                                    result,
                                                    &error);

    if (error) {
        g_printerr ("Failed to connect: %s\n", error->message);
        g_error_free (error);
        g_main_loop_quit (c->loop);
        return;
    }

    c->connected = TRUE;
    g_print ("Connected!\n");

    g_signal_connect (c->ws, "message",
                      G_CALLBACK (on_websocket_message), c);
    g_signal_connect (c->ws, "closed",
                      G_CALLBACK (on_websocket_closed), c);
    g_signal_connect (c->ws, "error",
                      G_CALLBACK (on_websocket_error), c);

    print_help ();
    g_print ("> ");
    fflush (stdout);

    setup_stdin (c);
}

static void
gpop_client_connect (GpopClient *c)
{
    g_print ("Connecting to %s...\n", c->url);

    SoupMessage *msg = soup_message_new (SOUP_METHOD_GET, c->url);
    if (!msg) {
        g_printerr ("Invalid URL: %s\n", c->url);
        g_main_loop_quit (c->loop);
        return;
    }

    soup_session_websocket_connect_async (c->session,
                                          msg,
                                          NULL,
                                          NULL,
                                          G_PRIORITY_DEFAULT,
                                          NULL,
                                          on_websocket_connected,
                                          c);
}

static GpopClient *
gpop_client_new (const gchar *url)
{
    GpopClient *c = g_new0 (GpopClient, 1);

    c->loop = g_main_loop_new (NULL, FALSE);
    c->session = soup_session_new ();
    c->url = g_strdup (url);
    c->connected = FALSE;
    c->ws = NULL;
    c->stdin_channel = NULL;
    c->stdin_watch_id = 0;

    return c;
}

static void
gpop_client_free (GpopClient *c)
{
    if (!c) return;

    if (c->stdin_watch_id > 0) {
        g_source_remove (c->stdin_watch_id);
    }

    if (c->stdin_channel) {
        g_io_channel_unref (c->stdin_channel);
    }

    if (c->ws) {
        if (soup_websocket_connection_get_state (c->ws) == SOUP_WEBSOCKET_STATE_OPEN) {
            soup_websocket_connection_close (c->ws, SOUP_WEBSOCKET_CLOSE_NORMAL, NULL);
        }
        g_object_unref (c->ws);
    }

    g_clear_object (&c->session);
    g_main_loop_unref (c->loop);
    g_free (c->url);
    g_free (c);
}

gint
main (gint argc, gchar *argv[])
{
    const gchar *url = DEFAULT_URL;

    if (argc > 1) {
        url = argv[1];
    }

    client = gpop_client_new (url);
    gpop_client_connect (client);
    g_main_loop_run (client->loop);
    gpop_client_free (client);

    return 0;
}
