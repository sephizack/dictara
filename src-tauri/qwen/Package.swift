// swift-tools-version:6.2
import PackageDescription

let package = Package(
    name: "DictaraQwen",
    platforms: [.macOS(.v14)],
    products: [.executable(name: "dictara-qwen", targets: ["DictaraQwen"])],
    dependencies: [
        .package(url: "https://github.com/Blaizzy/mlx-audio-swift.git", exact: "0.1.5"),
        .package(url: "https://github.com/ml-explore/mlx-swift-lm.git", exact: "3.32.3"),
        .package(url: "https://github.com/huggingface/swift-transformers.git", exact: "1.3.5")
    ],
    targets: [
        .executableTarget(
            name: "DictaraQwen",
            dependencies: [
                .product(name: "MLXAudioCore", package: "mlx-audio-swift"),
                .product(name: "MLXAudioSTT", package: "mlx-audio-swift"),
                .product(name: "MLXLLM", package: "mlx-swift-lm"),
                .product(name: "MLXLMCommon", package: "mlx-swift-lm"),
                .product(name: "MLXHuggingFace", package: "mlx-swift-lm"),
                .product(name: "Tokenizers", package: "swift-transformers")
            ]
        )
    ]
)