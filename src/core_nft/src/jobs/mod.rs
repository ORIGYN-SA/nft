mod upload_garbage_collector;

pub(crate) fn start() {
    upload_garbage_collector::start_job();
}
