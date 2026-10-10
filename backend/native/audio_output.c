#include <gst/gst.h>

#define PACKAGE "simpleVoiceover"

typedef struct {
    GstBin parent;
    GstElement *output;
    GstStructure *properties;
    gboolean synchronize;
    gboolean preview;
    gchar *client;
    gint64 offset;
} SvoiceSink;

typedef struct {
    GstBinClass parent;
} SvoiceSinkClass;

G_DEFINE_TYPE(SvoiceSink, svoice_sink, GST_TYPE_BIN)

static GstStaticPadTemplate sink_template = GST_STATIC_PAD_TEMPLATE(
    "sink", GST_PAD_SINK, GST_PAD_ALWAYS, GST_STATIC_CAPS("audio/x-raw"));

enum {
    PROP_STREAM = 1,
    PROP_CLIENT,
    PROP_SYNC,
    PROP_OFFSET
};

static void update_sync(SvoiceSink *sink) {
    const char *role = sink->properties
        ? gst_structure_get_string(sink->properties, "media.role")
        : NULL;
    gboolean webaudio = role && g_str_equal(role, "webaudio");
    g_object_set(sink->output, "sync", webaudio ? FALSE : sink->synchronize, NULL);
    if (!sink->preview && webaudio)
        g_object_set(sink->output, "buffer-time", (gint64)100000, NULL);
}

static void update_output(SvoiceSink *sink) {
    const char *role = sink->properties
        ? gst_structure_get_string(sink->properties, "media.role")
        : NULL;
    gboolean preview = role && g_str_equal(role, "video");
    if (preview != sink->preview) {
        GstElement *output = gst_element_factory_make(
            preview ? "fakesink" : "pulsesink",
            preview ? "preview-output" : "pulse-output");
        if (output) {
            if (preview)
                g_object_set(output, "enable-last-sample", FALSE, "async", FALSE, NULL);
            gst_bin_add(GST_BIN(sink), output);
            GstPad *ghost = gst_element_get_static_pad(GST_ELEMENT(sink), "sink");
            GstPad *target = gst_element_get_static_pad(output, "sink");
            gst_ghost_pad_set_target(GST_GHOST_PAD(ghost), target);
            gst_object_unref(target);
            gst_object_unref(ghost);
            gst_element_set_state(sink->output, GST_STATE_NULL);
            gst_bin_remove(GST_BIN(sink), sink->output);
            sink->output = output;
            sink->preview = preview;
            g_object_set(output, "ts-offset", sink->offset, NULL);
            update_sync(sink);
            gst_element_sync_state_with_parent(output);
        }
    }
    if (!sink->preview) {
        g_object_set(sink->output, "stream-properties", sink->properties, NULL);
        if (sink->client)
            g_object_set(sink->output, "client-name", sink->client, NULL);
    }
}

static void set_property(GObject *object, guint id, const GValue *value,
                         GParamSpec *spec) {
    SvoiceSink *sink = (SvoiceSink *)object;
    if (!sink->output)
        return;
    switch (id) {
    case PROP_STREAM:
        if (sink->properties)
            gst_structure_free(sink->properties);
        sink->properties = g_value_dup_boxed(value);
        if (sink->properties) {
            const char *role = gst_structure_get_string(sink->properties, "media.role");
            if (role && (g_str_equal(role, "webaudio") || g_str_equal(role, "video")))
                gst_structure_set(sink->properties, "application.id", G_TYPE_STRING,
                    g_str_equal(role, "webaudio") ? "dev.simplevoiceover.studio"
                                                  : "dev.simplevoiceover.studio.preview", NULL);
        }
        update_output(sink);
        update_sync(sink);
        break;
    case PROP_CLIENT:
        g_free(sink->client);
        sink->client = g_value_dup_string(value);
        if (!sink->preview)
            g_object_set_property(G_OBJECT(sink->output), "client-name", value);
        break;
    case PROP_SYNC:
        sink->synchronize = g_value_get_boolean(value);
        update_sync(sink);
        break;
    case PROP_OFFSET:
        sink->offset = g_value_get_int64(value);
        g_object_set_property(G_OBJECT(sink->output), "ts-offset", value);
        break;
    default:
        G_OBJECT_WARN_INVALID_PROPERTY_ID(object, id, spec);
    }
}

static void get_property(GObject *object, guint id, GValue *value,
                         GParamSpec *spec) {
    SvoiceSink *sink = (SvoiceSink *)object;
    if (!sink->output)
        return;
    switch (id) {
    case PROP_STREAM:
        g_value_set_boxed(value, sink->properties);
        break;
    case PROP_CLIENT:
        g_value_set_string(value, sink->client);
        break;
    case PROP_SYNC:
        g_value_set_boolean(value, sink->synchronize);
        break;
    case PROP_OFFSET:
        g_object_get_property(G_OBJECT(sink->output), "ts-offset", value);
        break;
    default:
        G_OBJECT_WARN_INVALID_PROPERTY_ID(object, id, spec);
    }
}

static void finalize(GObject *object) {
    SvoiceSink *sink = (SvoiceSink *)object;
    if (sink->properties)
        gst_structure_free(sink->properties);
    g_free(sink->client);
    G_OBJECT_CLASS(svoice_sink_parent_class)->finalize(object);
}

static void svoice_sink_class_init(SvoiceSinkClass *klass) {
    GObjectClass *object = G_OBJECT_CLASS(klass);
    object->set_property = set_property;
    object->get_property = get_property;
    object->finalize = finalize;
    g_object_class_install_property(object, PROP_STREAM, g_param_spec_boxed(
        "stream-properties", "Stream properties", "PulseAudio stream properties",
        GST_TYPE_STRUCTURE, G_PARAM_READWRITE));
    g_object_class_install_property(object, PROP_CLIENT, g_param_spec_string(
        "client-name", "Client name", "PulseAudio client name",
        NULL, G_PARAM_READWRITE));
    g_object_class_install_property(object, PROP_SYNC, g_param_spec_boolean(
        "sync", "Synchronize", "Synchronize media to the pipeline clock",
        TRUE, G_PARAM_READWRITE));
    g_object_class_install_property(object, PROP_OFFSET, g_param_spec_int64(
        "ts-offset", "Timestamp offset", "Output timestamp offset",
        G_MININT64, G_MAXINT64, 0, G_PARAM_READWRITE));
    gst_element_class_add_static_pad_template(GST_ELEMENT_CLASS(klass), &sink_template);
    gst_element_class_set_static_metadata(GST_ELEMENT_CLASS(klass),
        "simpleVoiceover audio output", "Sink/Audio",
        "PulseAudio output with WebAudio clock synchronization", "simpleVoiceover");
}

static void svoice_sink_init(SvoiceSink *sink) {
    sink->synchronize = TRUE;
    sink->output = gst_element_factory_make("pulsesink", "pulse-output");
    if (!sink->output)
        return;
    gst_bin_add(GST_BIN(sink), sink->output);
    GstPad *pad = gst_element_get_static_pad(sink->output, "sink");
    gst_element_add_pad(GST_ELEMENT(sink), gst_ghost_pad_new("sink", pad));
    gst_object_unref(pad);
}

static gboolean plugin_init(GstPlugin *plugin) {
    return gst_element_register(plugin, "svoiceaudiosink", GST_RANK_PRIMARY + 100,
                                svoice_sink_get_type());
}

GST_PLUGIN_DEFINE(GST_VERSION_MAJOR, GST_VERSION_MINOR, svoiceaudio,
                  "simpleVoiceover audio output", plugin_init, "1.0", "MIT/X11",
                  PACKAGE, "https://github.com/arconw/simpleVoiceover")
