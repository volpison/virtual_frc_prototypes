//! Choosing how finished frames are shown (vsync or not) from what the surface offers.

pub(super) fn choose_present_mode(
    available: &[wgpu::PresentMode],
    prefer_vsync: bool,
) -> wgpu::PresentMode {
    if prefer_vsync {
        if available.contains(&wgpu::PresentMode::Fifo) {
            return wgpu::PresentMode::Fifo;
        }
        if available.contains(&wgpu::PresentMode::FifoRelaxed) {
            return wgpu::PresentMode::FifoRelaxed;
        }
    }
    if available.contains(&wgpu::PresentMode::Immediate) {
        return wgpu::PresentMode::Immediate;
    }
    if available.contains(&wgpu::PresentMode::Mailbox) {
        return wgpu::PresentMode::Mailbox;
    }
    if available.contains(&wgpu::PresentMode::Fifo) {
        return wgpu::PresentMode::Fifo;
    }
    available[0]
}
