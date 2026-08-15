/*
 * Stub for tflite::MaybeCreateXNNPACKDelegate(int).
 *
 * libPixelFree_mac.a bundles TensorFlow Lite's register.cc which references
 * this factory, but the XNNPACK delegate objects are not included in the
 * archive (the vendor's Xcode demo resolves it via a second library that is
 * not shipped for macOS). Returning an empty delegate pointer matches
 * TFLite's own behavior when XNNPACK is compiled out — the interpreter then
 * runs on the plain CPU kernels.
 */
#include <memory>

struct TfLiteDelegate;

namespace tflite {

using TfLiteDelegatePtr = std::unique_ptr<TfLiteDelegate, void (*)(TfLiteDelegate*)>;

TfLiteDelegatePtr MaybeCreateXNNPACKDelegate(int /*num_threads*/) {
    return TfLiteDelegatePtr(nullptr, [](TfLiteDelegate*) {});
}

}  // namespace tflite
