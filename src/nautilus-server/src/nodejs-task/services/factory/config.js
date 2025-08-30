const config = {
  embedding: {
    providers: {
      ollama: {
        defaultBatchSize: 10,
        maxRetries: 3,
        timeout: 300000
      }
    }
  },
  vectorDb: {
    providers: {
      qdrant: {
        defaultBatchSize: 100,
        maxRetries: 3,
        timeout: 300000
      }
    }
  },
  refinement: {
    providers: {
      chat: {
        sortByDate: true,
        filterEmptyMessages: true
      }
    }
  },
  blockchain: {
    providers: {
      sui: {
        network: "testnet",
        maxRetries: 3,
        timeout: 300000
      },
      walrus: {
        maxRetries: 3,
        timeout: 300000
      },
      seal: {
        threshold: 2,
        maxRetries: 3,
        timeout: 300000
      }
    }
  }
};

module.exports = config;