#ifndef BVH_HPP
#define BVH_HPP

#include <chrono>
#include <iosfwd>
#include <optional>
#include <span>
#include <string_view>
#include <system_error>

#include "../../target/include/bvh_anim/bvh_anim.h"

namespace bvh {

static_assert(
    __cplusplus >= 202001L,
    "The bvh c++ wrappers are only compatible with c++ 20 or later");

class bvh;
class joint;
struct channel;
struct point;

struct point final {
public:
    constexpr point() noexcept = default;
    constexpr point(float x_, float y_, float z_) noexcept:
        x(x_), y(y_), z(z_) {}

    constexpr point(bvh_Offset offst) noexcept:
        x(offst.offset_x), y(offst.offset_y), z(offst.offset_z) {}

    float x = 0.0f, y = 0.0f, z = 0.0f;
};

enum class channel_type {
    x_position = X_POSITION,
    y_position = Y_POSITION,
    z_position = Z_POSITION,
    x_rotation = X_ROTATION,
    y_rotation = Y_ROTATION,
    z_rotation = Z_ROTATION,
};

struct channel final {
public:
    channel() = delete;
    constexpr channel(channel_type ty, size_t idx) noexcept: type(ty), index(idx) {}
    channel(bvh_Channel ch);

    channel_type type;
    size_t index;
};

/// Represents a `joint` in the herirarchy of a `bvh` file.
class joint final {
public:
    joint() = delete;

    joint(const joint&) = delete;
    joint(joint&&) = delete;
    joint& operator=(const joint&) = delete;
    joint& operator=(joint&&) = delete;

    ~joint() = default;

    /// Get the name of the `joint`.
    std::string_view name() const noexcept;
    /// Get the `joint`'s end site if present, otherwise return `nullopt`.
    std::optional<point> end_site() const noexcept;
    /// Get the `offset` of the joint.
    point offset() const noexcept;
    /// Get the array of `channel`s belonging to the joint.
    std::span<const channel> channels() const noexcept;
    /// Get the depth in the skeleton of the joint. The root joint has a depth
    /// of `0`.
    size_t depth() const noexcept;

private:
    friend class bvh;

    joint(bvh_Joint j): inner(j) {}

    bvh_Joint inner;
};

class bvh final {
public:
    constexpr bvh() noexcept = default;
    explicit bvh(std::ostream& os);
    explicit bvh(std::string_view sv);
    explicit bvh(const bvh& b);
    explicit bvh(bvh_BvhFile f): inner(f) {}

    bvh(bvh&& b) noexcept;

    bvh& operator=(const bvh& b);
    bvh& operator=(bvh&& b) noexcept;

    ~bvh() noexcept;

    std::chrono::duration<double> frame_time() const noexcept;
    void set_frame_time(std::chrono::duration<double> new_frame_time) noexcept;

    friend bool operator==(const bvh& b0, const bvh& b1) noexcept;
    friend std::ostream& operator>>(std::ostream& os, bvh& b);
    friend std::istream& operator<<(std::istream& is, const bvh& b);

private:
    bvh_BvhFile inner = {0};
};

constexpr inline bool operator==(const point& p0, const point& p1) noexcept {
    return (p0.x == p1.x) && (p0.y == p1.y) && (p0.z == p1.z);
}

constexpr inline bool operator!=(const point& p0, const point& p1) noexcept {
    return !operator==(p0, p1);
}

bool operator==(const bvh& b0, const bvh& b1) noexcept;

inline bool operator!=(const bvh& b0, const bvh& b1) noexcept {
    return !operator==(b0, b1);
}

std::ostream& operator>>(std::ostream& os, bvh& b);
std::istream& operator<<(std::istream& is, const bvh& b);

}; // namespace bvh

#endif // BVH_HPP
