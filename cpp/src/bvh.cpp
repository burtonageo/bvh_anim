#include "bvh/bvh.hpp"

#include <iostream>
#include <exception>

bvh::channel::channel(bvh_Channel ch) {
    index = ch.channel_index;
    switch(ch.channel_type) {
    case X_POSITION: {
        type = channel_type::x_position;
        break;
    }
    case Y_POSITION: {
        type = channel_type::y_position;
        break;
    }
    case Z_POSITION: {
        type = channel_type::z_position;
        break;
    }
    case X_ROTATION: {
        type = channel_type::x_rotation;
        break;
    }
    case Y_ROTATION: {
        type = channel_type::y_rotation;
        break;
    }
    case Z_ROTATION: {
        type = channel_type::z_rotation;
        break;
    }
    default: {
        throw std::logic_error("Attempting to construct a channel type with an invalid enum value");
    }
    }
}

std::string_view bvh::joint::name() const noexcept {
    return bvh_joint_get_name(&inner);
}

std::optional<bvh::point> bvh::joint::end_site() const noexcept {
    if (bvh_joint_has_end_site(&inner)) {
        return std::make_optional(point(bvh_joint_get_end_site(&inner)));
    }
    return std::nullopt;
}

bvh::point bvh::joint::offset() const noexcept {
    return point(bvh_joint_get_offset(&inner));
}

std::span<const bvh::channel> bvh::joint::channels() const noexcept {
    const size_t num_channels = bvh_joint_get_num_channels(&inner);
    const bvh_Channel* channels = bvh_joint_get_channels(&inner);
    return std::span(reinterpret_cast<const channel*>(channels), num_channels);
}

size_t bvh::joint::depth() const noexcept {
    return bvh_joint_get_depth(&inner);
}

std::chrono::duration<double> bvh::bvh::frame_time() const noexcept {
    return std::chrono::duration<double>(bvh_file_get_frame_time(&inner));
}

void bvh::bvh::set_frame_time(std::chrono::duration<double> new_frame_time) noexcept {
    bvh_file_set_frame_time(&inner, new_frame_time.count());
}

bool bvh::operator==(const bvh::bvh& b0, const bvh::bvh& b1) noexcept {
    return bvh_file_equal(&b0.inner, &b1.inner);
}

std::ostream& operator>>(std::ostream& os, bvh::bvh& b) {
    return os;
}

std::istream& bvh::operator<<(std::istream& is, const bvh::bvh& b) {
    return is;
}
