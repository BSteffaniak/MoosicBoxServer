ALTER TABLE track_sizes ADD COLUMN audio_bitrate INTEGER DEFAULT NULL;
ALTER TABLE track_sizes ADD COLUMN overall_bitrate INTEGER DEFAULT NULL;
ALTER TABLE track_sizes ADD COLUMN bit_depth INTEGER DEFAULT NULL;
ALTER TABLE track_sizes ADD COLUMN sample_rate INTEGER DEFAULT NULL;
ALTER TABLE track_sizes ADD COLUMN channels INTEGER DEFAULT NULL;
