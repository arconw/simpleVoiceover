#include <gst/gst.h>

int main(int argc, char **argv) {
    gst_init(NULL, NULL);
    g_assert_cmpint(argc, ==, 2);
    GError *error = NULL;
    GstPlugin *plugin = gst_plugin_load_file(argv[1], &error);
    g_assert_no_error(error);
    g_assert_nonnull(plugin);
    GstElement *sink = gst_element_factory_make("svoiceaudiosink", NULL);
    g_assert_nonnull(sink);
    GstElement *pulse = gst_bin_get_by_name(GST_BIN(sink), "pulse-output");
    g_assert_nonnull(pulse);

    gboolean sync = FALSE;
    gboolean provide_clock = FALSE;
    g_object_get(pulse, "sync", &sync, NULL);
    g_assert_true(sync);
    GstStructure *properties = gst_structure_new(
        "properties", "media.role", G_TYPE_STRING, "webaudio", NULL);
    g_object_set(sink, "stream-properties", properties, "sync", TRUE,
                 "client-name", "Audio test", "ts-offset", (gint64)123456, NULL);
    gint64 offset = 0;
    gint64 buffer_time = 0;
    gchar *client = NULL;
    g_object_get(pulse, "sync", &sync, "ts-offset", &offset,
                 "buffer-time", &buffer_time, "client-name", &client, NULL);
    g_assert_false(sync);
    g_object_get(pulse, "provide-clock", &provide_clock, NULL);
    g_assert_true(provide_clock);
    g_assert_cmpint(offset, ==, 123456);
    g_assert_cmpint(buffer_time, ==, 100000);
    g_assert_cmpstr(client, ==, "Audio test");
    g_free(client);
    GstStructure *output_properties = NULL;
    g_object_get(pulse, "stream-properties", &output_properties, NULL);
    g_assert_cmpstr(gst_structure_get_string(output_properties, "application.id"),
                   ==, "dev.simplevoiceover.studio");
    g_assert_false(gst_structure_has_field(properties, "application.id"));
    gst_structure_free(output_properties);

    gst_structure_set(properties, "media.role", G_TYPE_STRING, "video", NULL);
    g_object_set(sink, "stream-properties", properties, NULL);
    gst_object_unref(pulse);
    pulse = gst_bin_get_by_name(GST_BIN(sink), "preview-output");
    g_assert_nonnull(pulse);
    g_assert_null(gst_bin_get_by_name(GST_BIN(sink), "pulse-output"));
    g_assert_cmpstr(gst_plugin_feature_get_name(GST_PLUGIN_FEATURE(
                       gst_element_get_factory(pulse))), ==, "fakesink");
    g_object_get(pulse, "sync", &sync, "ts-offset", &offset, NULL);
    g_assert_true(sync);
    g_assert_cmpint(offset, ==, 123456);
    g_assert_null(gst_element_provide_clock(pulse));
    g_object_get(sink, "stream-properties", &output_properties, "client-name", &client, NULL);
    g_assert_cmpstr(client, ==, "Audio test");
    g_free(client);
    g_assert_cmpstr(gst_structure_get_string(output_properties, "application.id"),
                   ==, "dev.simplevoiceover.studio.preview");
    gst_structure_free(output_properties);
    g_object_set(sink, "sync", FALSE, NULL);
    g_object_get(pulse, "sync", &sync, NULL);
    g_assert_false(sync);
    g_object_set(sink, "sync", TRUE, "stream-properties", NULL, NULL);
    gst_object_unref(pulse);
    pulse = gst_bin_get_by_name(GST_BIN(sink), "pulse-output");
    g_assert_nonnull(pulse);
    g_object_get(pulse, "sync", &sync, NULL);
    g_assert_true(sync);
    g_object_get(pulse, "provide-clock", &provide_clock, NULL);
    g_assert_true(provide_clock);

    gst_structure_free(properties);
    gst_object_unref(pulse);
    gst_object_unref(sink);
    gst_object_unref(plugin);
    return 0;
}
